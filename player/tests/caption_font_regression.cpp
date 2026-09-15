#include <aribcaption/aribcaption.hpp>
#include <vector>
#include <iostream>
#include <string>
using namespace aribcaption;
Caption fixture() {
    Caption c;c.pts=1000;c.wait_duration=10000;c.plane_width=960;c.plane_height=540;c.iso6392_language_code=ThreeCC("por");
    CaptionRegion r;r.x=60;r.y=430;r.width=840;r.height=44;
    int x=r.x;
    for(auto ch:std::string("MIRINHO, VOCE VAI SER O HERDEIRO")) {
        CaptionChar cc;cc.type=CaptionCharType::kText;cc.codepoint=ch;cc.x=x;cc.y=r.y;
        cc.char_width=18;cc.char_height=36;cc.char_horizontal_spacing=4;cc.char_vertical_spacing=8;
        cc.char_horizontal_scale=cc.char_vertical_scale=1;cc.text_color=ColorRGBA(255,255,255,255);cc.back_color=ColorRGBA(0,0,0,255);
        r.chars.push_back(cc);x+=22;
    }
    c.regions.push_back(r);return c;
}
std::vector<uint8_t> pixels(Renderer& r) {
    RenderResult out;auto status=r.Render(1000,out);
    if(out.images.empty()) throw std::runtime_error("no rendered image");
    const auto& b=out.images.front().bitmap;return {b.begin(),b.end()};
}
int main() {
    Context c;Renderer changed(c),fresh(c);
    if(!changed.Initialize() || !fresh.Initialize())return 2;
    for(auto* r:{&changed,&fresh}) {r->SetFrameSize(1280,720);r->SetMergeRegionImages(true);r->AppendCaption(fixture());}
    changed.SetDefaultFontFamily({"Arial"},true);const auto before=pixels(changed);
    changed.SetDefaultFontFamily({"Consolas"},true);const auto after=pixels(changed);
    fresh.SetDefaultFontFamily({"Consolas"},true);const auto expected=pixels(fresh);
    if(before==expected){std::cerr<<"Fixture does not distinguish fonts\n";return 2;}
    if(after!=expected){std::cerr<<"Font change reused stale glyph metrics/font face\n";return 1;}
    std::cout<<"Font change matches a fresh Consolas renderer\n";
}
