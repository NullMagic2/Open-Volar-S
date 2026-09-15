import io
import json
from pathlib import Path
import struct
import tempfile
import unittest

import usb_trace as u
import compare_firmware as fw


def usb(data, ep=2, completion=False, device=5, status=0, transfer=3, irp=1, extra=b''):
    return struct.pack('<HQIHBHHBBI',27+len(extra),irp,status,9,int(completion),1,device,ep,transfer,len(data))+extra+data


def request(seq=0, cmd=0x22, payload=b'\x01'):
    b=bytes([5+len(payload),0,cmd,seq])+payload
    return b+u.checksum(b).to_bytes(2,'big')


def response(seq=0, status=0, payload=b'\x03\x00\x03\x00'):
    b=bytes([4+len(payload),seq,status])+payload
    return b+u.checksum(b).to_bytes(2,'big')


def pcap(data, endian='<', nano=False):
    magic={('<',False):b'\xd4\xc3\xb2\xa1',('>',False):b'\xa1\xb2\xc3\xd4',
           ('<',True):b'\x4d\x3c\xb2\xa1',('>',True):b'\xa1\xb2\x3c\x4d'}[endian,nano]
    out=magic+struct.pack(endian+'HHIIII',2,4,0,0,65535,249)
    for i,d in enumerate(data):
        out+=struct.pack(endian+'IIII',1,i*1000,len(d),len(d))+d
    return out


def block(kind,body,endian='<'):
    body+=b'\x00'*((-len(body))%4)
    return struct.pack(endian+'II',kind,len(body)+12)+body+struct.pack(endian+'I',len(body)+12)


def ng(data,endian='<',tsresol=6,offset=0):
    out=block(0x0a0d0d0a,struct.pack(endian+'IHHq',0x1a2b3c4d,1,0,-1),endian)
    opts=struct.pack(endian+'HH',9,1)+bytes([tsresol])+b'\0'*3
    opts+=struct.pack(endian+'HHq',14,8,offset)
    out+=block(1,struct.pack(endian+'HHI',249,0,65535)+opts,endian)
    for i,d in enumerate(data):
        out+=block(6,struct.pack(endian+'IIIII',0,0,1000000+i*1000,len(d),len(d))+d,endian)
    return out


class CaptureTests(unittest.TestCase):
    def setUp(self):
        self.tmp=tempfile.TemporaryDirectory()
        self.path=Path(self.tmp.name)/'capture.pcapng'
    def tearDown(self):
        self.tmp.cleanup()
    def load(self,data):
        self.path.write_bytes(data)
        return list(u.packets(self.path))

    def test_pcap_endianness_and_nanoseconds(self):
        for endian in '<>':
            for nano in (False,True):
                packets=self.load(pcap([usb(request()),usb(response(),0x81,True)],endian,nano))
                self.assertEqual(packets[1].time_ns,10**9+1000*(1 if nano else 1000))
                self.assertEqual(u.usb_packet(packets[0])['device'],5)

    def test_pcapng_sections_interfaces_and_clocks(self):
        packets=self.load(ng([usb(request())],'<',9,7)+ng([usb(request())],'>',0x80|10))
        self.assertEqual(packets[0].time_ns,7*10**9+1000000)
        self.assertEqual(packets[1].section,1)
        self.assertEqual(packets[1].time_ns,1000000*10**9//1024)

    def test_request_response_pair_and_submission_dedup(self):
        data=[usb(request()),usb(b'',completion=True),usb(b'',0x81),usb(response(),0x81,True)]
        self.path.write_bytes(ng(data))
        out=Path(self.tmp.name)/'commands.jsonl'
        summary=u.analyze(self.path,(0,0,1,5),out)
        events=[json.loads(s) for s in out.read_text().splitlines()]
        self.assertEqual(summary['counts']['candidate_pairs'],1)
        self.assertEqual(len(events),2)
        self.assertEqual(events[1]['request_packet'],1)
        self.assertEqual(events[1]['latency_ns'],3000000)

    def test_never_pairs_other_devices_or_failed_usb_completions(self):
        data=[usb(request()),usb(response(),0x81,True,device=6),usb(response(),0x81,True,status=0xc0000001)]
        self.path.write_bytes(ng(data))
        summary=u.analyze(self.path,(0,0,1,5),Path(self.tmp.name)/'c.jsonl')
        self.assertNotIn('candidate_pairs',summary['counts'])
        self.assertEqual(summary['counts']['usb_errors'],1)

    def test_descriptor_inventory(self):
        descriptor=bytes.fromhex('1201000200000040ca0765b8000101020301')
        self.path.write_bytes(ng([usb(descriptor,0x80,True,transfer=2,extra=b'\x03')]))
        self.assertEqual(u.inventory(self.path)['devices'][0]['descriptor_ids'],['07ca:b865'])

    def test_checksums_short_errors_and_scatter_redaction(self):
        self.assertEqual(request().hex(),'0600220001ffdc')
        self.assertEqual(u.frame(response(status=5,payload=b''),True)['device_status'],5)
        self.assertIsNone(u.frame(request()[:-1]+b'\x00',False))
        decoded=u.frame(request(cmd=0x29,payload=bytes.fromhex('030000014100030212bf')),False)
        self.assertEqual(decoded['regions'],[{'address':0x4100,'length':3}])
        self.assertNotIn('payload',decoded)

    def test_bad_block_truncation_and_unbounded_length(self):
        good=ng([usb(request())])
        for bad in (good[:-1],good[:-4]+b'\x00'*4,pcap([])+struct.pack('<IIII',0,0,0xffffffff,0xffffffff)):
            with self.assertRaises(ValueError): self.load(bad)

    def test_truncated_urb_is_not_decoded(self):
        data=bytearray(usb(request()))
        struct.pack_into('<I',data,23,99)
        self.path.write_bytes(ng([data]))
        summary=u.analyze(self.path,(0,0,1,5),Path(self.tmp.name)/'c.jsonl')
        self.assertEqual(summary['counts']['truncated_bulk_packets'],1)
        self.assertNotIn('candidate_request',summary['counts'])

    def test_firmware_parser_bounds(self):
        for bad in (b'',bytes.fromhex('03000001410000'),bytes.fromhex('03000001ffff020000')):
            with self.assertRaises(ValueError): fw.parse_scatter(bad)
        report=fw.parse_scatter(bytes.fromhex('030000014100030212bf'))
        self.assertEqual(report['regions'][0]['reset_ljmp_target'],0x12bf)


if __name__=='__main__': unittest.main()
