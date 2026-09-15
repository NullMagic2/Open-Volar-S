#include <aribcaption/aribcaption.hpp>
#include <fstream>
#include <iostream>
#include <vector>
#include <string>
#include <cstdint>
using namespace aribcaption;
std::vector<uint8_t> group(uint8_t id,const std::vector<uint8_t>& body) {
    std::vector<uint8_t> g={uint8_t(id<<2),0,0,uint8_t(body.size()>>8),uint8_t(body.size())};
    g.insert(g.end(),body.begin(),body.end());uint16_t crc=0xffff;
    for(auto b:g){crc^=uint16_t(b)<<8;for(int i=0;i<8;i++)crc=crc&0x8000?(crc<<1)^0x1021:crc<<1;}
    g.push_back(crc>>8);g.push_back(crc);g.insert(g.begin(),{0x80,0xff,0xf0});return g;
}
std::vector<uint8_t> statement(const std::string& text) {
    std::vector<uint8_t> body={0,0,0,uint8_t(text.size()+5),0x1f,0x20,0,0,uint8_t(text.size())};
    body.insert(body.end(),text.begin(),text.end());return group(1,body);
}
// Exercise every Latin letter through PES decoding and the character records
// consumed by the renderer. Expected values do not depend on decoder tables.
std::string utf8(uint32_t cp) {
    if(cp<0x80)return std::string(1,char(cp));
    if(cp<0x800)return {char(0xc0|(cp>>6)),char(0x80|(cp&63))};
    return {char(0xe0|(cp>>12)),char(0x80|((cp>>6)&63)),char(0x80|(cp&63))};
}
bool all_latin_letters_preserve_case() {
    std::vector<std::pair<uint8_t,uint32_t>> cases;
    for(uint32_t c='A';c<='Z';++c)cases.emplace_back(c,c);
    for(uint32_t c='a';c<='z';++c)cases.emplace_back(c,c);
    for(uint32_t c=0xc0;c<=0xfe;++c)if(c!=0xd7&&c!=0xf7)cases.emplace_back(c,c);
    for(auto c:std::vector<std::pair<uint8_t,uint32_t>>{
        {0xa6,0x160},{0xa8,0x161},{0xb4,0x17d},{0xb8,0x17e},
        {0xbc,0x152},{0xbd,0x153},{0xbe,0x178},{0xad,0xff}})cases.push_back(c);
    for(auto [encoded,expected]:cases) {
        Context ctx;Decoder dec(ctx);DecodeResult out;
        if(!dec.Initialize())return false;
        auto management=group(0,{0,1,0,'p','o','r',0x80,0,0,0});
        dec.Decode(management.data(),management.size(),0,out);
        auto packet=statement(std::string("a")+char(encoded)+"Z");
        if(dec.Decode(packet.data(),packet.size(),1,out)!=DecodeStatus::kGotCaption||!out.caption)return false;
        std::vector<uint32_t> glyphs;
        for(const auto& region:out.caption->regions)
            for(const auto& ch:region.chars)glyphs.push_back(ch.codepoint);
        if(out.caption->text!="a"+utf8(expected)+"Z"||glyphs!=std::vector<uint32_t>{'a',expected,'Z'}) {
            std::cerr<<"Case preservation failed for broadcast byte "<<unsigned(encoded)<<"\n";return false;
        }
    }
    std::cout<<cases.size()<<" Latin letters preserve case in decoded text and renderer glyph records\n";
    return true;
}
int main(int argc,char** argv) {
    if(argc==1&&!all_latin_letters_preserve_case())return 6;
    Context ctx;Decoder dec(ctx);if(!dec.Initialize())return 2;DecodeResult out;
    auto management=group(0,{0,1,0,'p','o','r',0x80,0,0,0});dec.Decode(management.data(),management.size(),0,out);
    if(argc==1){
        auto data=statement(std::string("s\xf3 s\xd3 t\xe1 T\xc1 n\xe3o N\xc3O voc\xea VOC\xca a\xe7\xe3o A\xc7\xc3O"));
        auto status=dec.Decode(data.data(),data.size(),1000,out);
        if(status!=DecodeStatus::kGotCaption||!out.caption)return 3;
        std::cout<<out.caption->text<<"\n";
        return out.caption->text==u8"só sÓ tá TÁ não NÃO você VOCÊ ação AÇÃO"?0:1;
    }
    std::ifstream in(argv[1],std::ios::binary);uint32_t size=0;int64_t pts=0;
    while(in.read(reinterpret_cast<char*>(&size),4)) {
        if(size>65536)return 4;std::vector<uint8_t> data(size);if(!in.read(reinterpret_cast<char*>(data.data()),size))return 5;
        auto status=dec.Decode(data.data(),data.size(),pts++,out);
        if(status==DecodeStatus::kGotCaption&&out.caption){std::cout<<out.caption->text<<"\n";}
    }
}
