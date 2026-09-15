//! Broadcast H.264 parser feeding the existing Vulkan decoder and GPU textures.
#![forbid(unsafe_code)]
use crate::{
    VulkanDecoderError, VulkanDevice,
    codec::h264::{H264Codec, H264VkParameters},
    parser::reference_manager::{
        DecodeInformation, PictureInfo, ReferenceId, ReferencePictureInfo,
    },
    vulkan_decoder::{ImageModifiers, VulkanDecoder},
    wrappers::VideoSessionParameters,
};
use ash::vk;
use broadcast_parser::{Event, Parser, Picture};
use std::{collections::HashMap, sync::Arc};

pub struct Frame {
    pub texture: wgpu::Texture,
    pub display_aspect: (u32,u32),
    pub pts: i64,
    pub duration: i64,
    pub interlaced: bool,
    pub top_first: bool,
    pub bt709: bool,
    pub full: bool,
}
pub struct Decoder {
    parser: Option<Parser>,
    gpu: VulkanDecoder<'static>,
    config: u64,
    stream_shape: Option<(u32, u32, u32, u32, u32)>,
    started: bool,
    waiting: HashMap<u64, Frame>,
    output: Vec<Frame>,

    pub parsed_pictures: u64,
    pub skipped_startup_pictures: u64,
    pub decoded_pictures: u64,
    pub decoded_fields: u64,
}
fn invalid(e: impl std::fmt::Display) -> VulkanDecoderError {
    VulkanDecoderError::InvalidInputData(e.to_string())
}
impl Decoder {
    pub fn new(device: &Arc<VulkanDevice>) -> Result<Self, VulkanDecoderError> {
        if !cfg!(feature = "experimental-broadcast-gpu") {
            return Err(invalid(
                "Experimental broadcast GPU decoding is disabled pending validation",
            ));
        }
        if device.queues.transfer.family_index == device.queues.wgpu.family_index
            || device
                .queues
                .h264_decode
                .as_ref()
                .is_some_and(|q| q.family_index == device.queues.wgpu.family_index)
        {
            return Err(invalid(
                "This preview requires separate Vulkan video, transfer and presentation queue families",
            ));
        }
        let gpu = VulkanDecoder::new(
            Arc::new(device.decoding_device()?),
            Default::default(),
            ImageModifiers {
                additional_queue_index: device.queues.transfer.family_index,
                create_flags: Default::default(),
                usage_flags: Default::default(),
            },
        )?;
        let parser = Parser::new().map_err(invalid)?;
        Ok(Self {
            parser: Some(parser),
            gpu,
            config: 0,
            stream_shape: None,
            started: false,
            waiting: HashMap::new(),
            output: Vec::new(),
            parsed_pictures: 0,
            skipped_startup_pictures: 0,
            decoded_pictures: 0,
            decoded_fields: 0,
        })
    }
    pub fn decode(
        &mut self,
        bytes: &[u8],
        pts: Option<i64>,
    ) -> Result<Vec<Frame>, VulkanDecoderError> {
        self.process(bytes, pts, false)
    }
    pub fn flush(&mut self) -> Result<Vec<Frame>, VulkanDecoderError> {
        self.process(&[], None, true)
    }
    fn process(
        &mut self,
        bytes: &[u8],
        pts: Option<i64>,
        eos: bool,
    ) -> Result<Vec<Frame>, VulkanDecoderError> {
        let mut parser = self
            .parser
            .take()
            .ok_or_else(|| invalid("Reentrant decoder call"))?;
        let result = parser.parse(bytes, pts, eos, |event| {
            match event {
                Event::Picture(picture) => self.picture(&picture).map_err(|e| e.to_string())?,
                Event::Display { id, pts } => {
                    if let Some(mut frame) = self.waiting.remove(&id) {
                        frame.pts = pts;
                        self.output.push(frame);
                    }
                }
            }
            Ok(())
        });
        self.parser = Some(parser);
        result.map_err(invalid)?;
        Ok(std::mem::take(&mut self.output))
    }
    fn picture(&mut self, p: &Picture<'_>) -> Result<(), VulkanDecoderError> {
        self.parsed_pictures += 1;
        let caps = &self
            .gpu
            .decoding_device
            .profile_capabilities
            .video_capabilities;
        if p.width == 0
            || p.height == 0
            || p.width > p.coded_width
            || p.height > p.coded_height
            || p.coded_width > caps.max_coded_extent.width
            || p.coded_height > caps.max_coded_extent.height
            || p.bottom > 1
            || p.field > 1
            || p.second > 1
            || p.width % 2 != 0
            || p.height % 2 != 0
        {
            return Err(invalid("Invalid broadcast dimensions or field metadata"));
        }
        // Tuning may start mid-GOP. Do not submit pictures with unavailable references.
        if !self.started {
            // Open-GOP broadcasts can start with independent non-IDR intra pictures.
            // Preserve the actual bitstream IDR flag; only open the startup gate.
            if !broadcast_parser::independent_start(p.idr!=0,p.intra!=0,p.second!=0,p.reference_count as usize) {
                self.skipped_startup_pictures += 1;
                return Ok(());
            }
            self.started = true;
        }
        let sps = p.sps;
        let pps = p.pps;
        if self.config != p.config {
            // Metadata sizes drive resource allocation. The original native SPS/PPS,
            // including all scaling lists, are passed unchanged to the GPU below.
            let metadata = resource_metadata(sps, p.vui)?;
            let shape = (
                p.coded_width,
                p.coded_height,
                sps.flags.frame_mbs_only_flag(),
                sps.profile_idc,
                metadata.max_num_ref_frames,
            );
            if self.stream_shape != Some(shape) {
                if self.stream_shape.is_some() && p.idr == 0 {
                    return Err(invalid("Stream format changed before an IDR picture"));
                }
                let profile =
                    crate::codec::h264::parameters::H264DecodeProfileInfo::from_sps_decode(
                        &metadata,
                        Default::default(),
                    )?;
                let device = self
                    .gpu
                    .decoding_device
                    .vulkan_device
                    .broadcast_decoding_device(&profile.profile_info.profile_info)?;
                // Recreate only at an independent picture; discard old reference mappings.
                let vkdevice = device.vulkan_device.clone();
                self.gpu = VulkanDecoder::new(
                    Arc::new(device),
                    Default::default(),
                    ImageModifiers {
                        additional_queue_index: vkdevice.queues.transfer.family_index,
                        create_flags: Default::default(),
                        usage_flags: Default::default(),
                    },
                )?;
                self.stream_shape = Some(shape);
            }
            let caps = &self
                .gpu
                .decoding_device
                .profile_capabilities
                .video_capabilities;
            if p.coded_width < caps.min_coded_extent.width
                || p.coded_height < caps.min_coded_extent.height
                || p.coded_width > caps.max_coded_extent.width
                || p.coded_height > caps.max_coded_extent.height
                || metadata.max_num_ref_frames + 1 > caps.max_dpb_slots
            {
                return Err(invalid("Broadcast exceeds Vulkan decode capabilities"));
            }
            self.gpu.process_sps(&metadata)?;
            self.gpu.ensure_broadcast_session()?;
            let session = self.gpu.video_session_resources.as_mut().unwrap();
            session.parameters_manager.parameters =
                Arc::new(VideoSessionParameters::new::<H264Codec>(
                    self.gpu.decoding_device.device.clone(),
                    session.video_session.session,
                    H264VkParameters {
                        sps: vec![*sps],
                        pps: vec![*pps],
                    },
                    None,
                    None,
                )?);
            self.config = p.config;
        }
        let refs = p.references[..p.reference_count as usize]
            .iter()
            .map(|r| ReferencePictureInfo {
                id: ReferenceId(r.id as usize),
                field_mask: r.flags & 3,
                LongTermPicNum: if r.flags & 4 != 0 {
                    Some(r.frame_num as u64)
                } else {
                    None
                },
                non_existing: false,
                FrameNum: r.frame_num as u16,
                PicOrderCnt: r.poc,
            })
            .collect::<Vec<_>>();
        let active_count: usize = refs
            .iter()
            .map(|r| if r.field_mask == 3 { 2 } else { 1 })
            .sum();
        if active_count
            > self
                .gpu
                .video_session_resources
                .as_ref()
                .unwrap()
                .parameters
                .max_active_references as usize
        {
            return Err(invalid("Too many active H264 reference fields"));
        }
        let current = ReferenceId(p.id as usize);
        if (p.idr == 0 || p.second != 0)
            && refs
                .iter()
                .any(|r| !self.gpu.reference_id_to_dpb_slot_index.contains_key(&r.id))
        {
            return Err(invalid(
                "Broadcast references a missing decoded picture; retune to recover",
            ));
        }
        let obsolete = self
            .gpu
            .reference_id_to_dpb_slot_index
            .keys()
            .copied()
            .filter(|id| !refs.iter().any(|r| r.id == *id) && !(p.second != 0 && *id == current))
            .collect::<Vec<_>>();
        for id in obsolete {
            if let Some(slot) = self.gpu.reference_id_to_dpb_slot_index.remove(&id) {
                self.gpu
                    .video_session_resources
                    .as_mut()
                    .unwrap()
                    .decoding_images
                    .free_reference_picture(slot);
            }
        }
        let mut picture = vk::native::StdVideoDecodeH264PictureInfo {
            flags: vk::native::StdVideoDecodeH264PictureInfoFlags {
                _bitfield_align_1: [],
                __bindgen_padding_0: [0; 3],
                _bitfield_1: vk::native::StdVideoDecodeH264PictureInfoFlags::new_bitfield_1(
                    0, 0, 0, 0, 0, 0,
                ),
            },
            seq_parameter_set_id: 0,
            pic_parameter_set_id: 0,
            reserved1: 0,
            reserved2: 0,
            frame_num: 0,
            idr_pic_id: 0,
            PicOrderCnt: [0, 0],
        };
        picture.flags.set_complementary_field_pair(p.second);
        picture.flags.set_field_pic_flag(p.field);
        picture.flags.set_bottom_field_flag(p.bottom);
        picture.flags.set_IdrPicFlag(p.idr);
        picture.flags.set_is_intra(p.intra);
        picture.flags.set_is_reference(p.reference);
        picture.seq_parameter_set_id = sps.seq_parameter_set_id;
        picture.pic_parameter_set_id = pps.pic_parameter_set_id;
        picture.frame_num = p.frame_num as u16;
        picture.idr_pic_id = u16::try_from(p.idr_pic_id).map_err(|_|invalid("Invalid IDR picture identifier"))?;
        picture.PicOrderCnt = p.poc;
        let bytes = p.bytes;
        let slices = p.slices;
        let info = DecodeInformation {
            reference_list_l0: Some(refs),
            reference_list_l1: None,
            rbsp_bytes: bytes.to_vec(),
            slice_indices: slices.iter().map(|x| *x as usize).collect(),
            header: None,
            native_picture: Some(picture),
            second_field: p.second != 0,
            sps_id: sps.seq_parameter_set_id,
            pps_id: pps.pic_parameter_set_id,
            pts: None,
            picture_info: PictureInfo {
                field_mask: if p.field == 0 { 0 } else { 1 << p.bottom },
                used_for_long_term_reference: false,
                non_existing: false,
                FrameNum: p.frame_num as u16,
                PicOrderCnt_for_decoding: p.poc,
                PicOrderCnt_as_reference_pic: p.poc,
            },
        };
        let mut submission = self.gpu.do_decode(
            &info,
            current,
            p.idr != 0 && p.second == 0,
            p.reference != 0,
        )?;
        submission.decode_result.frame.cropped_extent = vk::Extent2D {
            width: p.width,
            height: p.height,
        };
        let texture = submission.output_to_wgpu_texture()?.frame;
        let duration = if p.rate_num > 0 && p.rate_den > 0 {
            10_000_000 * p.rate_den as i64 / p.rate_num as i64
        } else {
            333667
        };
        // Both fields of PAFF pictures share one DPB image. Replace the first-field
        // output only after the complementary field has been decoded.
        self.waiting.insert(
            p.id,
            Frame {
                texture,
                display_aspect: p.vui.map(|v| broadcast_parser::display_aspect(p.width,p.height,
                    if v.flags.aspect_ratio_info_present_flag()!=0 {v.aspect_ratio_idc} else {0},(v.sar_width,v.sar_height)))
                    .unwrap_or((p.width,p.height)),
                pts: 0,
                duration: duration * (2 + p.repeat as i64) / 2,
                interlaced: p.progressive == 0,
                top_first: p.top_first != 0,
                bt709: p.matrix == 1,
                full: p.full != 0,
            },
        );
        if self.waiting.len() > 32 {
            return Err(invalid("Broadcast display queue exceeded its bound"));
        }
        self.decoded_pictures += 1;
        self.decoded_fields += p.field as u64;
        Ok(())
    }
}
fn resource_metadata(
    s: &vk::native::StdVideoH264SequenceParameterSet,
    vui: Option<&vk::native::StdVideoH264SequenceParameterSetVui>,
) -> Result<h264_reader::nal::sps::SeqParameterSet, VulkanDecoderError> {
    use h264_reader::nal::sps::*;
    if s.chroma_format_idc != 1 || s.bit_depth_luma_minus8 != 0 || s.bit_depth_chroma_minus8 != 0 {
        return Err(invalid("Unsupported broadcast pixel format"));
    }
    let buffering = vui.map(|v| v.max_dec_frame_buffering as u32).unwrap_or(0);
    if s.max_num_ref_frames > 16 || buffering > 16 {
        return Err(invalid("Invalid H.264 reference capacity"));
    }
    Ok(SeqParameterSet {
        profile_idc: (s.profile_idc as u8).into(),
        constraint_flags: 0.into(),
        level_idc: crate::codec::h264::parameters::vk_to_h264_level_idc(s.level_idc)?,
        seq_parameter_set_id: SeqParamSetId::from_u32(s.seq_parameter_set_id as u32)
            .map_err(|e| invalid(format!("{e:?}")))?,
        chroma_info: ChromaInfo {
            chroma_format: ChromaFormat::YUV420,
            ..Default::default()
        },
        log2_max_frame_num_minus4: s.log2_max_frame_num_minus4,
        pic_order_cnt: PicOrderCntType::TypeZero {
            log2_max_pic_order_cnt_lsb_minus4: s.log2_max_pic_order_cnt_lsb_minus4,
        },
        max_num_ref_frames: (s.max_num_ref_frames as u32).max(buffering).min(16),
        gaps_in_frame_num_value_allowed_flag: s.flags.gaps_in_frame_num_value_allowed_flag() != 0,
        pic_width_in_mbs_minus1: s.pic_width_in_mbs_minus1,
        pic_height_in_map_units_minus1: s.pic_height_in_map_units_minus1,
        frame_mbs_flags: if s.flags.frame_mbs_only_flag() != 0 {
            FrameMbsFlags::Frames
        } else {
            FrameMbsFlags::Fields {
                mb_adaptive_frame_field_flag: s.flags.mb_adaptive_frame_field_flag() != 0,
            }
        },
        direct_8x8_inference_flag: s.flags.direct_8x8_inference_flag() != 0,
        frame_cropping: None,
        vui_parameters: None,
    })
}
