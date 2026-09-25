// Narrow, exception-contained adapter. Context owns decoder and renderer lifetimes.
#include <aribcaption/aribcaption.hpp>
#include <vector>
#include "caption_layout.hpp"
#include <cstdint>
using namespace aribcaption;
struct CaptionEngine {
    Context context;
    Decoder decoder{context};
    Renderer renderer{context};
    RenderResult result;
    int width=0,height=0;
};
struct CaptionImage {const uint8_t* data;size_t length;int width,height,stride,x,y;};
extern "C" {
void* a865r_cc_new(int profile) noexcept {
    try {
        auto p=std::make_unique<CaptionEngine>();
        if(!p->decoder.Initialize(EncodingScheme::kAuto,CaptionType::kCaption,profile==0x12?Profile::kProfileC:Profile::kProfileA) || !p->renderer.Initialize())return nullptr;
        p->renderer.SetMergeRegionImages(true);
        p->renderer.SetStoragePolicy(CaptionStoragePolicy::kUpperLimitCount,128);
        p->renderer.SetForceStrokeText(true);
        for(auto lang:{ThreeCC("por"),ThreeCC("spa"),ThreeCC("eng")})
#ifdef _WIN32
            p->renderer.SetLanguageSpecificFontFamily(lang,{"Consolas","Courier New"});
#else
            p->renderer.SetLanguageSpecificFontFamily(lang,{"DejaVu Sans Mono","Liberation Mono"});
#endif
        return p.release();
    }catch(...){return nullptr;}
}
void a865r_cc_free(void* p) noexcept {delete static_cast<CaptionEngine*>(p);}
int a865r_cc_decode(void* ptr,const uint8_t* data,size_t length,int64_t pts) noexcept {
    if(!ptr||!data||length==0||length>65536)return 0;
    try{auto& p=*static_cast<CaptionEngine*>(ptr);DecodeResult out;
        auto status=p.decoder.Decode(data,length,pts,out);
        if(status==DecodeStatus::kGotCaption && out.caption){
            center_latin_caption(*out.caption);
            if(!p.renderer.AppendCaption(std::move(*out.caption)))return 0;
        }
        return static_cast<int>(status);
    }catch(...){return 0;}
}
int a865r_cc_render(void* ptr,int64_t pts,int width,int height,CaptionImage* image) noexcept {
    if(!ptr||!image||width<1||height<1||width>7680||height>4320)return 0;
    try{auto& p=*static_cast<CaptionEngine*>(ptr);
        if(p.width!=width||p.height!=height){p.renderer.SetFrameSize(width,height);p.width=width;p.height=height;}
        else if(p.renderer.TryRender(pts)==RenderStatus::kGotImageUnchanged)return 3;
        auto status=p.renderer.Render(pts,p.result);
        if(status==RenderStatus::kGotImage || status==RenderStatus::kGotImageUnchanged){
            if(p.result.images.empty())return 1;
            const auto& im=p.result.images.front();
            *image={im.bitmap.data(),im.bitmap.size(),im.width,im.height,im.stride,im.dst_x,im.dst_y};
            return 2;
        }
        return static_cast<int>(status);
    }catch(...){return 0;}
}
void a865r_cc_invalidate(void* ptr) noexcept {if(ptr){auto& p=*static_cast<CaptionEngine*>(ptr);p.width=0;p.height=0;}}
void a865r_cc_flush(void* ptr) noexcept {
    try{if(ptr){auto& p=*static_cast<CaptionEngine*>(ptr);p.decoder.Flush();p.renderer.Flush();p.result={};}}catch(...){}
}
}
