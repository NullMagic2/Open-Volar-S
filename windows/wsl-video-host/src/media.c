/* Windows-side demux/mux and hardware fallback. Video is never decoded on the
 * CPU and never re-encoded: the wire format is timestamped NV12 in NUT, with
 * original audio packets. See FFmpeg's hw_decode API example for the hardware
 * frame transfer contract. Diagnostics go to stderr; stdout is binary media. */
#include <stdio.h>
#include <stdint.h>
#include <stdlib.h>
#include <io.h>
#include <fcntl.h>
#include <windows.h>
#include <libavformat/avformat.h>
#include <libavcodec/avcodec.h>
#include <libavutil/hwcontext.h>
#include <libavutil/imgutils.h>
#include <libavutil/error.h>

typedef int (*Decode)(void *, void *, const uint8_t *, int, int64_t);
typedef int (*Output)(void *, const uint8_t *, int);
typedef struct Host {
    AVFormatContext *input, *output;
    AVCodecContext *decoder;
    AVBufferRef *hw_device;
    enum AVPixelFormat hw_format;
    int video, *map, map_count, backend, frames, pictures, max_frames, confirmed;
    int64_t last_pts, discard_before, output_origin;
    void *opaque;
    Decode vulkan;
} Host;
static const AVRational clock_base = {1,10000000};
static const char *names[] = {"Vulkan Video", "D3D11VA", "DXVA2"};
/* Hide temporary EOF from the TS demuxer. Returning EOF and retrying outside
 * av_read_frame flushes partial PES/access units and corrupts a growing source. */
static int read_growing(void *opaque,uint8_t *buffer,int size) {
    AVIOContext *file=opaque;
    for(;;) {
        int result=avio_read_partial(file,buffer,size);
        if(result!=AVERROR_EOF && result!=0)return result;
        file->eof_reached=0;file->error=0;
        Sleep(20);
    }
}
static int64_t seek_growing(void *opaque,int64_t offset,int whence) {
    AVIOContext *file=opaque;
    if(whence==AVSEEK_SIZE)return avio_size(file);
    return avio_seek(file,offset,whence);
}
static void diagnostic(const char *operation, int result) {
    char error[AV_ERROR_MAX_STRING_SIZE];
    av_strerror(result,error,sizeof(error));
    fprintf(stderr,"[DEBUG]: %s: %s\n",operation,error);
}
static enum AVPixelFormat require_hardware(AVCodecContext *codec, const enum AVPixelFormat *formats) {
    Host *host = codec->opaque;
    for (; *formats != AV_PIX_FMT_NONE; ++formats)
        if (*formats == host->hw_format) return *formats;
    /* Returning NONE is deliberate: never silently choose software H.264. */
    return AV_PIX_FMT_NONE;
}
static int open_hardware(Host *host, int backend) {
    avcodec_free_context(&host->decoder);
    av_buffer_unref(&host->hw_device);
    host->backend = backend;
    AVCodecParameters *parameters = host->input->streams[host->video]->codecpar;
    const AVCodec *codec = avcodec_find_decoder(parameters->codec_id);
    if (!codec) return AVERROR_DECODER_NOT_FOUND;
    enum AVHWDeviceType type = backend == 1 ? AV_HWDEVICE_TYPE_D3D11VA : AV_HWDEVICE_TYPE_DXVA2;
    host->hw_format = AV_PIX_FMT_NONE;
    for (int i=0;;i++) {
        const AVCodecHWConfig *config = avcodec_get_hw_config(codec,i);
        if (!config) break;
        if (config->device_type == type && (config->methods & AV_CODEC_HW_CONFIG_METHOD_HW_DEVICE_CTX)) {
            host->hw_format = config->pix_fmt;break;
        }
    }
    if (host->hw_format == AV_PIX_FMT_NONE) return AVERROR(ENOSYS);
    int result = av_hwdevice_ctx_create(&host->hw_device,type,NULL,NULL,0);
    if (result<0) return result;
    host->decoder = avcodec_alloc_context3(codec);
    if (!host->decoder) return AVERROR(ENOMEM);
    if ((result=avcodec_parameters_to_context(host->decoder,parameters))<0) return result;
    host->decoder->hw_device_ctx = av_buffer_ref(host->hw_device);
    host->decoder->opaque = host;
    host->decoder->get_format = require_hardware;
    host->decoder->thread_count = 1;
    host->decoder->pkt_timebase = host->input->streams[host->video]->time_base;
    return avcodec_open2(host->decoder,codec,NULL);
}
static int fallback(Host *host) {
    for (int next=host->backend+1;next<=2;next++) {
        fprintf(stderr,"[DEBUG]: %s not available; trying %s\n",names[host->backend],names[next]);
        int result=open_hardware(host,next);
        if (result>=0) {host->confirmed=0;return 0;}
        diagnostic(names[next],result);
    }
    fprintf(stderr,"[ERROR]: No hardware video decoder is available; software video decoding is disabled.\n");
    return AVERROR(ENOSYS);
}
int ovs_host_frame(void *mux,const uint8_t *bytes,int length,int width,int height,
    int64_t pts,int64_t duration,int interlaced,int top_first) {
    Host *host=mux;
    AVStream *video=host->output->streams[0];
    host->pictures++;
    if (pts<host->discard_before) return 0;
    if (host->max_frames>0 && host->frames>=host->max_frames) return 0;
    if (width!=video->codecpar->width || height!=video->codecpar->height ||
        width<=0 || height<=0 || width>8192 || height>8192 ||
        length!=(int64_t)width*height*3/2) return AVERROR_INVALIDDATA;
    AVPacket *packet=av_packet_alloc();
    if (!packet) return AVERROR(ENOMEM);
    int result=av_new_packet(packet,length);
    if (result<0) {av_packet_free(&packet);return result;}
    memcpy(packet->data,bytes,length);
    packet->stream_index=0;
    packet->pts=packet->dts=av_rescale_q(pts-host->output_origin,clock_base,video->time_base);
    packet->duration=av_rescale_q(duration,clock_base,video->time_base);
    packet->flags=AV_PKT_FLAG_KEY;
    /* Keep the stream field order for raw frames. The Linux bridge also passes
     * source field order explicitly because rawvideo packet flags cannot carry it. */
    (void)interlaced; (void)top_first;
    result=av_interleaved_write_frame(host->output,packet);
    av_packet_free(&packet);
    if (result<0) return result;
    host->last_pts=pts;
    host->frames++;
    if (!host->confirmed) {
        fprintf(stderr,"[DEBUG]: hardware decoder active: %s (%dx%d NV12)\n",names[host->backend],width,height);
        host->confirmed=1;
    }
    return 0;
}
static int drain_hardware(Host *host) {
    AVFrame *gpu=av_frame_alloc(), *cpu=av_frame_alloc();
    if (!gpu || !cpu) {av_frame_free(&gpu);av_frame_free(&cpu);return AVERROR(ENOMEM);}
    int result=0;
    while ((result=avcodec_receive_frame(host->decoder,gpu))>=0) {
        if (gpu->format!=host->hw_format) {result=AVERROR(ENOSYS);break;}
        result=av_hwframe_transfer_data(cpu,gpu,0);
        if (result<0) break;
        if (cpu->format!=AV_PIX_FMT_NV12) {result=AVERROR(ENOSYS);break;}
        int size=av_image_get_buffer_size(AV_PIX_FMT_NV12,cpu->width,cpu->height,1);
        uint8_t *pixels=av_malloc(size);
        if (!pixels) {result=AVERROR(ENOMEM);break;}
        result=av_image_copy_to_buffer(pixels,size,(const uint8_t *const *)cpu->data,cpu->linesize,
            AV_PIX_FMT_NV12,cpu->width,cpu->height,1);
        int64_t pts=gpu->best_effort_timestamp;
        if (pts==AV_NOPTS_VALUE) pts=gpu->pts;
        AVRational tb=host->input->streams[host->video]->time_base;
        int64_t duration=gpu->duration>0?av_rescale_q(gpu->duration,tb,clock_base):333667;
        pts=pts==AV_NOPTS_VALUE?host->last_pts+duration:av_rescale_q(pts,tb,clock_base);
        if (result>=0) result=ovs_host_frame(host,pixels,size,cpu->width,cpu->height,pts,duration,
            !!(gpu->flags&AV_FRAME_FLAG_INTERLACED),!!(gpu->flags&AV_FRAME_FLAG_TOP_FIELD_FIRST));
        av_free(pixels);av_frame_unref(gpu);av_frame_unref(cpu);
        if (result<0) break;
    }
    av_frame_free(&gpu);av_frame_free(&cpu);
    return result==AVERROR(EAGAIN)||result==AVERROR_EOF?0:result;
}
int ovs_host_run(const char *input,const char *output,int backend,int program,int max_frames,double seek,int follow,
    void *opaque,Decode decode,Output write_output,void *write_opaque) {
    _setmode(_fileno(stdin),_O_BINARY);_setmode(_fileno(stdout),_O_BINARY);
    av_log_set_level(AV_LOG_WARNING);
    Host host={0};host.video=-1;host.backend=backend;host.max_frames=max_frames;
    host.opaque=opaque;host.vulkan=decode;
    AVPacket *packet=NULL;int result=0;int selected_program=-1;
    AVIOContext *growing_file=NULL,*growing_io=NULL;
    AVDictionary *options=NULL;
    av_dict_set(&options,"probesize","2000000",0);
    av_dict_set(&options,"analyzeduration","2000000",0);
    if (follow) av_dict_set(&options,"skip_estimate_duration_from_pts","1",0);
    if (follow) {
        if((result=avio_open(&growing_file,input,AVIO_FLAG_READ))<0)goto done;
        uint8_t *buffer=av_malloc(64*1024);
        if(!buffer){result=AVERROR(ENOMEM);goto done;}
        growing_io=avio_alloc_context(buffer,64*1024,0,growing_file,read_growing,NULL,seek_growing);
        if(!growing_io){av_free(buffer);result=AVERROR(ENOMEM);goto done;}
        host.input=avformat_alloc_context();
        if(!host.input){result=AVERROR(ENOMEM);goto done;}
        host.input->pb=growing_io;host.input->flags|=AVFMT_FLAG_CUSTOM_IO;
    }
    result=avformat_open_input(&host.input,input,NULL,&options);av_dict_free(&options);
    if (result<0) goto done;
    if ((result=avformat_find_stream_info(host.input,NULL))<0) goto done;
    for (unsigned p=0;p<host.input->nb_programs;p++)
        if (program && host.input->programs[p]->program_num==program) selected_program=p;
    if (program && selected_program<0) {result=AVERROR_STREAM_NOT_FOUND;goto done;}
    host.map=av_malloc_array(host.input->nb_streams,sizeof(int));
    if (!host.map) {result=AVERROR(ENOMEM);goto done;}
    host.map_count=host.input->nb_streams;
    for (unsigned i=0;i<host.input->nb_streams;i++) {
        host.map[i]=-1;
        int belongs=selected_program<0;
        if (!belongs) {
            AVProgram *p=host.input->programs[selected_program];
            for(unsigned j=0;j<p->nb_stream_indexes;j++) if(p->stream_index[j]==i) belongs=1;
        }
        if (belongs && host.video<0 && host.input->streams[i]->codecpar->codec_type==AVMEDIA_TYPE_VIDEO) host.video=i;
    }
    if (host.video<0) {result=AVERROR_STREAM_NOT_FOUND;goto done;}
    AVStream *source=host.input->streams[host.video];
    if (selected_program<0) {
        for(unsigned p=0;p<host.input->nb_programs && selected_program<0;p++)
            for(unsigned j=0;j<host.input->programs[p]->nb_stream_indexes;j++)
                if(host.input->programs[p]->stream_index[j]==(unsigned)host.video)selected_program=p;
    }
    host.output_origin=host.input->start_time==AV_NOPTS_VALUE?0:
        av_rescale_q(host.input->start_time,AV_TIME_BASE_Q,clock_base);
    host.discard_before=host.output_origin;
    fprintf(stderr,"[DEBUG]: source duration: %.6f\n",host.input->duration==AV_NOPTS_VALUE?0.0:(double)host.input->duration/AV_TIME_BASE);
    fprintf(stderr,"[DEBUG]: source field order: %d\n",source->codecpar->field_order);
    if (seek>0) {
        int64_t origin=host.input->start_time==AV_NOPTS_VALUE?0:host.input->start_time;
        int64_t target=origin+(int64_t)(seek*AV_TIME_BASE);
        /* MPEG-TS timestamp seeking does not promise a decodable keyframe.
         * Retain a GOP preroll and discard decoded pictures before the target.
         * For early seeks restart at the file beginning, including its SPS/PPS. */
        int64_t preroll=target-10*AV_TIME_BASE;
        if(preroll<origin)preroll=origin;
        if ((result=avformat_seek_file(host.input,-1,INT64_MIN,preroll,preroll,AVSEEK_FLAG_BACKWARD))<0)goto done;
        host.discard_before=av_rescale_q(target,AV_TIME_BASE_Q,clock_base);
        host.output_origin=host.discard_before;
    }
    if ((result=avformat_alloc_output_context2(&host.output,NULL,"nut",output))<0) goto done;
    AVStream *video=avformat_new_stream(host.output,NULL);
    if(!video){result=AVERROR(ENOMEM);goto done;}
    video->time_base=clock_base;video->avg_frame_rate=source->avg_frame_rate;
    video->sample_aspect_ratio=source->sample_aspect_ratio;
    avcodec_parameters_copy(video->codecpar,source->codecpar);
    av_freep(&video->codecpar->extradata);video->codecpar->extradata_size=0;
    video->codecpar->codec_id=AV_CODEC_ID_RAWVIDEO;
    video->codecpar->codec_tag=MKTAG('N','V','1','2');
    video->codecpar->format=AV_PIX_FMT_NV12;video->codecpar->bit_rate=0;
    /* Frames have already been reordered into display order by the hardware
     * decoder. Carrying the H.264 B-frame delay into NUT makes the receiving
     * demuxer invent reordered DTS for an intra-only raw stream. */
    video->codecpar->video_delay=0;
    video->codecpar->profile=AV_PROFILE_UNKNOWN;
    video->codecpar->level=AV_LEVEL_UNKNOWN;
    video->codecpar->bits_per_coded_sample=12;
    for(unsigned i=0;i<host.input->nb_streams;i++) {
        AVStream *src=host.input->streams[i];
        if (src->codecpar->codec_type!=AVMEDIA_TYPE_AUDIO) continue;
        if(selected_program>=0) {
            AVProgram *p=host.input->programs[selected_program];int belongs=0;
            for(unsigned j=0;j<p->nb_stream_indexes;j++) if(p->stream_index[j]==i) belongs=1;
            if(!belongs) continue;
        }
        AVStream *dst=avformat_new_stream(host.output,NULL);
        if(!dst){result=AVERROR(ENOMEM);goto done;}
        avcodec_parameters_copy(dst->codecpar,src->codecpar);dst->codecpar->codec_tag=0;
        dst->time_base=src->time_base;av_dict_copy(&dst->metadata,src->metadata,0);
        host.map[i]=dst->index;
    }
    host.output->max_interleave_delta=500000;
    host.output->flags|=AVFMT_FLAG_FLUSH_PACKETS;
    if(write_output) {
        uint8_t *buffer=av_malloc(256*1024);
        if(!buffer){result=AVERROR(ENOMEM);goto done;}
        host.output->pb=avio_alloc_context(buffer,256*1024,1,write_opaque,NULL,write_output,NULL);
        if(!host.output->pb){av_free(buffer);result=AVERROR(ENOMEM);goto done;}
        host.output->flags|=AVFMT_FLAG_CUSTOM_IO;
    }else if ((result=avio_open(&host.output->pb,output,AVIO_FLAG_WRITE))<0) goto done;
    if ((result=avformat_write_header(host.output,NULL))<0) goto done;
    if(backend>0) {
        result=open_hardware(&host,backend);
        if(result<0) {diagnostic(names[backend],result);if((result=fallback(&host))<0)goto done;}
    } else if (source->codecpar->codec_id!=AV_CODEC_ID_H264) {
        fprintf(stderr,"[DEBUG]: Vulkan Video broadcast backend requires H.264.\n");
        if((result=fallback(&host))<0)goto done;
    }
    packet=av_packet_alloc();if(!packet){result=AVERROR(ENOMEM);goto done;}
    int without_frames=0;
    for (;;) {
        result=av_read_frame(host.input,packet);
        if (result==AVERROR_EOF && follow) {
            /* A growing recording is temporarily at EOF. Keep the demuxer and
             * partial PES/parser state and retry when capture appends more data. */
            if(host.input->pb){host.input->pb->eof_reached=0;host.input->pb->error=0;}
            Sleep(20);continue;
        }
        if(result<0)break;
        if(packet->stream_index==host.video) {
            int frames_before=host.pictures;
            if(host.backend==0) {
                int64_t pts=packet->pts==AV_NOPTS_VALUE?INT64_MIN:av_rescale_q(packet->pts,source->time_base,clock_base);
                result=decode(opaque,&host,packet->data,packet->size,pts);
                if(result==-2){result=AVERROR(EPIPE);break;}
                if(host.pictures==frames_before) without_frames++;else without_frames=0;
                if(without_frames>180) {fprintf(stderr,"[DEBUG]: Vulkan Video produced no frames for 180 packets.\n");result=-1;}
                if(result<0) {
                    if((result=fallback(&host))<0) break;
                    /* Continue from this packet. Hardware decoders wait for the
                     * next valid reference chain after a mid-stream transition. */
                }
            }
            if(host.backend>0) {
                result=avcodec_send_packet(host.decoder,packet);
                if(result==AVERROR(EAGAIN)) {result=drain_hardware(&host);if(result>=0)result=avcodec_send_packet(host.decoder,packet);}
                if(result>=0) result=drain_hardware(&host);
                if(result<0 && result!=AVERROR_INVALIDDATA) {
                    diagnostic(names[host.backend],result);
                    if((result=fallback(&host))<0) break;
                }
            }
        } else if(packet->stream_index>=0 && packet->stream_index<host.map_count && host.map[packet->stream_index]>=0) {
            AVStream *src=host.input->streams[packet->stream_index];
            if (packet->pts!=AV_NOPTS_VALUE && av_rescale_q(packet->pts,src->time_base,clock_base)<host.discard_before) {
                av_packet_unref(packet);continue;
            }
            packet->stream_index=host.map[packet->stream_index];
            int64_t origin=av_rescale_q(host.output_origin,clock_base,src->time_base);
            if(packet->pts!=AV_NOPTS_VALUE)packet->pts-=origin;
            if(packet->dts!=AV_NOPTS_VALUE)packet->dts-=origin;
            av_packet_rescale_ts(packet,src->time_base,host.output->streams[packet->stream_index]->time_base);
            result=av_interleaved_write_frame(host.output,packet);
            if(result<0) break;
        }
        av_packet_unref(packet);
        if(max_frames>0 && host.frames>=max_frames) {result=0;break;}
    }
    if(result==AVERROR_EOF) {
        result=0;
        if(host.backend==0) result=decode(opaque,&host,NULL,0,INT64_MIN);
        else {avcodec_send_packet(host.decoder,NULL);result=drain_hardware(&host);}
    }
    if(result>=0 && host.frames==0) {fprintf(stderr,"[ERROR]: Hardware decoder produced no frames.\n");result=AVERROR_INVALIDDATA;}
    if(result>=0)result=av_write_trailer(host.output);
    fprintf(stderr,"[DEBUG]: decoded frames: %d; hardware decoder: %s\n",host.frames,names[host.backend]);
done:
    if(result<0) diagnostic("Windows video bridge",result);
    av_packet_free(&packet);avcodec_free_context(&host.decoder);av_buffer_unref(&host.hw_device);
    av_free(host.map);avformat_close_input(&host.input);
    av_dict_free(&options);
    if(growing_io){av_freep(&growing_io->buffer);avio_context_free(&growing_io);}
    avio_closep(&growing_file);
    if(host.output) {
        if(write_output && host.output->pb){av_freep(&host.output->pb->buffer);avio_context_free(&host.output->pb);}
        else{avio_closep(&host.output->pb);}
        avformat_free_context(host.output);
    }
    return result;
}
