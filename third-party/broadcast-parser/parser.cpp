#include <cstdio>
#include <vector>
#include <cstring>
#include <stdexcept>
#include <mutex>
static std::recursive_mutex parser_mutex;
#include "vkvideo_parser/VulkanVideoParserIf.h"
#include "nvVulkanVideoParser.h"
class Bytes final:public VulkanBitstreamBuffer {
    int refs=0;
    std::vector<uint8_t> bytes;
    std::vector<uint32_t> markers;
    void range(VkDeviceSize offset,VkDeviceSize count) const {if(offset>bytes.size() || count>bytes.size()-offset)throw std::out_of_range("Bitstream buffer range");}
    static void capacity(VkDeviceSize n){if(n>64*1024*1024)throw std::length_error("Bitstream buffer limit");}
public:
    explicit Bytes(size_t size){capacity(size);bytes.resize(size);}
    int32_t AddRef() override {return ++refs;}
    int32_t Release() override {int n=--refs;if(!n)delete this;return n;}
    VkDeviceSize GetMaxSize() const override{return bytes.size();}
    VkDeviceSize GetOffsetAlignment() const override{return 1;}
    VkDeviceSize GetSizeAlignment() const override{return 1;}
    VkDeviceSize Resize(VkDeviceSize n,VkDeviceSize size,VkDeviceSize offset) override {
        capacity(n);range(offset,size);if(size>n)throw std::out_of_range("Bitstream resize");
        if(offset && size)memmove(bytes.data(),bytes.data()+offset,size);
        bytes.resize(n);return n;
    }
    VkDeviceSize Clone(VkDeviceSize n,VkDeviceSize size,VkDeviceSize offset,VkSharedBaseObj<VulkanBitstreamBuffer>& out) override {
        range(offset,size);if(size>n)throw std::out_of_range("Bitstream clone");
        auto p=new Bytes(n);memcpy(p->bytes.data(),bytes.data()+offset,size);out.Reset(p);return n;
    }
    int64_t MemsetData(uint32_t v,VkDeviceSize o,VkDeviceSize n) override {range(o,n);memset(bytes.data()+o,v,n);return n;}
    int64_t CopyDataToBuffer(uint8_t* d,VkDeviceSize dst,VkDeviceSize src,VkDeviceSize n) const override {range(src,n);if(!d)throw std::invalid_argument("Null output");memcpy(d+dst,bytes.data()+src,n);return n;}
    int64_t CopyDataToBuffer(VkSharedBaseObj<VulkanBitstreamBuffer>& d,VkDeviceSize dst,VkDeviceSize src,VkDeviceSize n) const override {range(src,n);return d->CopyDataFromBuffer(bytes.data(),src,dst,n);}
    int64_t CopyDataFromBuffer(const uint8_t* s,VkDeviceSize src,VkDeviceSize dst,VkDeviceSize n) override {range(dst,n);if(!s)throw std::invalid_argument("Null input");memcpy(bytes.data()+dst,s+src,n);return n;}
    int64_t CopyDataFromBuffer(const VkSharedBaseObj<VulkanBitstreamBuffer>& s,VkDeviceSize src,VkDeviceSize dst,VkDeviceSize n) override {range(dst,n);return s->CopyDataToBuffer(bytes.data(),dst,src,n);}
    uint8_t* GetDataPtr(VkDeviceSize o,VkDeviceSize& n) override {range(o,0);n=bytes.size()-o;return bytes.data()+o;}
    const uint8_t* GetReadOnlyDataPtr(VkDeviceSize o,VkDeviceSize& n) const override {range(o,0);n=bytes.size()-o;return bytes.data()+o;}
    void FlushRange(VkDeviceSize,VkDeviceSize) const override {}
    void InvalidateRange(VkDeviceSize,VkDeviceSize) const override {}
    VkBuffer GetBuffer() const override{return VK_NULL_HANDLE;}
    VkDeviceMemory GetDeviceMemory() const override{return VK_NULL_HANDLE;}
    uint32_t AddStreamMarker(uint32_t o) override {if(markers.size()>=65536)throw std::length_error("Slice limit");markers.push_back(o);return markers.size()-1;}
    uint32_t SetStreamMarker(uint32_t o,uint32_t i) override {if(i>=65536)throw std::length_error("Slice index");if(i>=markers.size())markers.resize(i+1);markers[i]=o;return i;}
    uint32_t GetStreamMarker(uint32_t i) const override{return markers.at(i);}
    uint32_t GetStreamMarkersCount() const override{return markers.size();}
    const uint32_t* GetStreamMarkersPtr(uint32_t i,uint32_t& n) const override {if(i>markers.size())throw std::out_of_range("Slice markers");n=markers.size()-i;return markers.data()+i;}
    uint32_t ResetStreamMarkers() override {markers.clear();return 0;}
};
struct Reference {
    uint64_t id; int32_t frame_num; uint32_t flags; int32_t poc[2];
};
struct Picture {
    uint64_t id,config;
    uint32_t field,bottom,second,progressive,top_first,repeat,reference,intra,idr;
    uint32_t coded_width,coded_height,width,height,rate_num,rate_den,matrix,full;
    uint32_t frame_num;int32_t poc[2];uint32_t reference_count;
    Reference references[17];
    const StdVideoH264SequenceParameterSet* sps;
    const StdVideoH264PictureParameterSet* pps;
    const uint8_t* bytes;size_t size;const uint32_t* slices;uint32_t slice_count;uint32_t idr_pic_id;
};
static_assert(sizeof(Picture)==560,"Rust/C++ picture ABI mismatch");
static_assert(sizeof(Reference)==24,"Rust/C++ reference ABI mismatch");
static_assert(sizeof(StdVideoH264SequenceParameterSet)==88,"SPS ABI mismatch");
static_assert(sizeof(StdVideoH264PictureParameterSet)==24,"PPS ABI mismatch");
using Callback=int(*)(void*,uint32_t,const void*,int64_t);
struct Pic final:vkPicBuffBase {uint64_t id=0;bool field=false;};
class Client final:public VkParserVideoDecodeClient {
    Pic pictures[32];
    uint64_t nextId=1,config=0;
    VkSharedBaseObj<StdVideoPictureParametersSet> lastSps,lastPps;
    VkParserSequenceInfo sequence{};
public:
    VkSharedBaseObj<VulkanVideoDecodeParser> parser;
    Callback callback=nullptr;void* context=nullptr;
    ~Client(){parser=nullptr;}
    int32_t BeginSequence(const VkParserSequenceInfo* s) override {sequence=*s;return 32;}
    bool AllocPictureBuffer(VkPicIf** p) override {
        for(auto& pic:pictures)if(pic.IsAvailable()){
            pic.id=nextId++;pic.AddRef();*p=&pic;return true;
        }return false;
    }
    bool DecodePicture(VkParserPictureData* p) override {
        const auto& h=p->CodecSpecific.h264;
        static_cast<Pic*>(p->pCurrPic)->field=p->field_pic_flag;
        if(lastSps.Get()!=h.pStdSps || lastPps.Get()!=h.pStdPps){
            lastSps.Reset(const_cast<StdVideoPictureParametersSet*>(h.pStdSps));
            lastPps.Reset(const_cast<StdVideoPictureParametersSet*>(h.pStdPps));config++;
        }
        Picture out{};out.id=static_cast<Pic*>(p->pCurrPic)->id;out.config=config;
        out.field=p->field_pic_flag;out.bottom=p->bottom_field_flag;out.second=p->second_field;
        out.progressive=p->progressive_frame;out.top_first=p->top_field_first;
        out.repeat=p->repeat_first_field;out.reference=p->ref_pic_flag;out.intra=p->intra_pic_flag;
        out.coded_width=p->PicWidthInMbs*16;out.coded_height=p->FrameHeightInMbs*16;
        out.width=sequence.nDisplayWidth;out.height=sequence.nDisplayHeight;
        out.rate_num=sequence.frameRate>>14;out.rate_den=sequence.frameRate&0x3fff;
        out.matrix=sequence.lMatrixCoefficients;out.full=sequence.uVideoFullRange;
        out.frame_num=h.frame_num;out.idr_pic_id=h.idr_pic_id;out.poc[0]=h.CurrFieldOrderCnt[0];out.poc[1]=h.CurrFieldOrderCnt[1];
        out.sps=h.pStdSps->GetStdH264Sps();out.pps=h.pStdPps->GetStdH264Pps();
        VkDeviceSize available=0;
        out.bytes=p->bitstreamData->GetReadOnlyDataPtr(p->bitstreamDataOffset,available);
        out.size=p->bitstreamDataLen;out.slice_count=p->numSlices;
        if(!out.bytes || out.size>available || !out.size || !out.slice_count)return false;
        uint32_t count=0;out.slices=p->bitstreamData->GetStreamMarkersPtr(p->firstSliceIndex,count);
        if(!out.slices || out.slice_count>count)return false;
        for(size_t i=0;i+3<out.size;i++)if(out.bytes[i]==0&&out.bytes[i+1]==0&&out.bytes[i+2]==1){auto type=out.bytes[i+3]&31;if(type==1 || type==5){out.idr=type==5;break;}}
        for(const auto& r:h.dpb)if(r.used_for_reference && r.pPicBuf && !r.not_existing){
            if(out.reference_count>=17)return false;
            auto& dst=out.references[out.reference_count++];dst.id=static_cast<Pic*>(r.pPicBuf)->id;
            dst.frame_num=r.FrameIdx;dst.flags=(static_cast<Pic*>(r.pPicBuf)->field?r.used_for_reference:0) | (r.is_long_term?4:0);
            dst.poc[0]=r.FieldOrderCnt[0];dst.poc[1]=r.FieldOrderCnt[1];
        }
        return callback && callback(context,1,&out,0)!=0;
    }
    bool UpdatePictureParameters(VkSharedBaseObj<StdVideoPictureParametersSet>&,VkSharedBaseObj<VkVideoRefCountBase>&) override{return true;}
    bool DisplayPicture(VkPicIf* p,int64_t pts) override {
        auto id=static_cast<Pic*>(p)->id;return callback && callback(context,2,&id,pts)!=0;
    }
    void UnhandledNALU(const uint8_t*,size_t) override{}
    VkDeviceSize GetBitstreamBuffer(VkDeviceSize n,VkDeviceSize,VkDeviceSize,const uint8_t* init,VkDeviceSize count,VkSharedBaseObj<VulkanBitstreamBuffer>& out) override {
        auto p=new Bytes(n);if(count)p->CopyDataFromBuffer(init,0,0,count);out.Reset(p);return n;
    }
};
extern "C" void* broadcast_create(){
    std::lock_guard<std::recursive_mutex> guard(parser_mutex);
    try {
        auto c=new Client;
        VkParserInitDecodeParameters init{};
        init.interfaceVersion=NV_VULKAN_VIDEO_PARSER_API_VERSION;init.pClient=c;
        init.defaultMinBufferSize=2*1024*1024;init.bufferOffsetAlignment=1;init.bufferSizeAlignment=1;
        init.referenceClockRate=10000000;init.errorThreshold=0;
        VkExtensionProperties version={VK_STD_VULKAN_VIDEO_CODEC_H264_DECODE_EXTENSION_NAME,VK_STD_VULKAN_VIDEO_CODEC_H264_DECODE_SPEC_VERSION};
        if(CreateVulkanVideoDecodeParser(VK_VIDEO_CODEC_OPERATION_DECODE_H264_BIT_KHR,&version,nullptr,0,&init,c->parser)!=VK_SUCCESS){delete c;return nullptr;}
        return c;
    }catch(...){return nullptr;}
}
extern "C" void broadcast_destroy(void* parser){std::lock_guard<std::recursive_mutex> guard(parser_mutex);delete static_cast<Client*>(parser);}
extern "C" int broadcast_parse(void* parser,const uint8_t* bytes,size_t size,int64_t pts,int valid,int eos,void* context,Callback callback){
    std::lock_guard<std::recursive_mutex> guard(parser_mutex);
    try{
        auto c=static_cast<Client*>(parser);c->context=context;c->callback=callback;
        VkParserBitstreamPacket p{};p.pByteStream=bytes;p.nDataLength=size;p.llPTS=pts;p.bPTSValid=valid!=0;p.bEOS=eos!=0;
        return c->parser->ParseByteStream(&p)?1:0;
    }catch(...){return 0;}
}
