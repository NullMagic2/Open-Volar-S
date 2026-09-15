// Readable, centered Latin subtitles; timing, colors, character identity and clear events are retained.
#pragma once
#include <aribcaption/caption.hpp>
#include <algorithm>
#include <cmath>
#include <map>
#include <vector>
inline void center_latin_caption(aribcaption::Caption& caption) {
    using namespace aribcaption;
    if(caption.plane_width<64 || caption.plane_height<64 || caption.regions.empty()) return;
    const auto language=caption.iso6392_language_code;
    if(language!=ThreeCC("por") && language!=ThreeCC("spa") && language!=ThreeCC("eng")) return;
    std::map<int,std::vector<CaptionChar>> rows;
    size_t count=0;
    int original_height=0;
    for(const auto& region:caption.regions) {
        if(region.is_ruby) return;
        for(const auto& ch:region.chars) {
            // Preserve broadcast-specific graphics and non-Latin layout untouched.
            if(ch.type!=CaptionCharType::kText || ch.codepoint<32 || ch.codepoint>255 || ++count>4096) return;
            rows[ch.y].push_back(ch);
            original_height=std::max(original_height,(int)std::lround(ch.char_height*ch.char_vertical_scale));
        }
    }
    if(!count) return;
    const int margin=std::max(8,caption.plane_width/20);
    const int max_width=caption.plane_width-2*margin;
    // Slightly larger text; wrapping still enforces the same safe area.
    int height=std::clamp((original_height*110+50)/100,12,std::max(12,caption.plane_height/16));
    std::vector<std::vector<CaptionChar>> lines;
    int advance=0;
    // Rewrap, shrinking only when needed to keep every line inside the safe area.
    for(;;) {
        advance=height/2+std::max(1,(int)std::ceil(height*.06f));
        const size_t capacity=std::max(1,max_width/advance);
        lines.clear();
        for(auto& [y,row]:rows) {
            std::stable_sort(row.begin(),row.end(),[](const auto& a,const auto& b){return a.x<b.x;});
            size_t begin=0;
            while(begin<row.size()) {
                while(begin<row.size() && row[begin].codepoint==' ') ++begin;
                if(begin==row.size()) break;
                size_t end=std::min(row.size(),begin+capacity);
                if(end<row.size()) {
                    size_t space=end;
                    while(space>begin && row[space].codepoint!=' ') --space;
                    if(space>begin) end=space;
                }
                size_t trimmed=end;
                while(trimmed>begin && row[trimmed-1].codepoint==' ') --trimmed;
                lines.emplace_back(row.begin()+begin,row.begin()+trimmed);
                begin=end;
            }
        }
        if(lines.size()*(height+std::max(3,height/5))<=size_t(caption.plane_height*4/5) || height<=8) break;
        --height;
    }
    const int line_height=height+std::max(3,height/5);
    const int bottom_margin=std::max(8,caption.plane_height/16);
    const int total_height=(int)lines.size()*line_height;
    if(total_height>caption.plane_height-bottom_margin) return;
    std::vector<CaptionRegion> centered;
    int y=caption.plane_height-bottom_margin-total_height;
    for(auto& line:lines) {
        CaptionRegion region;
        region.width=(int)line.size()*advance;
        region.height=line_height;
        region.x=(caption.plane_width-region.width)/2;
        region.y=y;
        int x=region.x;
        for(auto ch:line) {
            // Half-width Latin cells retain native glyph proportions in DirectWrite.
            ch.char_width=height/2;ch.char_height=height;
            ch.char_horizontal_scale=ch.char_vertical_scale=1.f;
            ch.char_horizontal_spacing=advance-ch.char_width;
            ch.char_vertical_spacing=line_height-height;
            ch.x=x;ch.y=y;region.chars.push_back(ch);x+=advance;
        }
        centered.push_back(std::move(region));y+=line_height;
    }
    caption.regions=std::move(centered);
}
