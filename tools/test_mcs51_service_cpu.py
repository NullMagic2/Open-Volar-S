import unittest
from mcs51_service_cpu import Cpu

class InstructionTests(unittest.TestCase):
 def execute(self,program,setup=None):
  code=bytearray(65536);present=bytearray(65536);code[0x100:0x100+len(program)]=program;present[0x100:0x100+len(program)]=bytes([1])*len(program)
  cpu=Cpu(code,present,bytes(65536))
  if setup:setup(cpu)
  return cpu.run(0x100)
 def test_call_return_uses_upper_internal_ram_not_sfr_aliases(self):
  # CALL 0x107; MOV A,#42; RET; NOP; MOV R7,#9; RET
  cpu=self.execute(bytes([0x12,1,7,0x74,42,0x22,0,0x7F,9,0x22]))
  self.assertEqual((cpu.a,cpu.r(7),cpu.d[0x81]),(42,9,0x80))
 def test_push_pop_preserve_a_register_across_call(self):
  cpu=self.execute(bytes([0x7F,9,0xC0,7,0x7F,11,0xD0,7,0x22]))
  self.assertEqual(cpu.r(7),9);self.assertEqual(cpu.d[0x81],0x80)
 def test_add_carry_and_subtract_borrow(self):
  for left in [0,1,127,128,254,255]:
   for right in [0,1,127,128,254,255]:
    for carry in [0,1]:
     for opcode in [0x34,0x94]:
      cpu=self.execute(bytes([0x74,left,0xD3 if carry else 0xC3,opcode,right,0x22]))
      expected=left+right+carry if opcode==0x34 else left-right-carry
      self.assertEqual(cpu.a,expected&255);self.assertEqual(cpu.c,int(expected>255 if opcode==0x34 else expected<0))
 def test_direct_register_operand_and_compare_carry(self):
  cpu=self.execute(bytes([0x7C,7,0x74,6,0xB5,4,2,0x74,0,0x22]))
  self.assertEqual((cpu.a,cpu.c),(6,1))
 def test_xdata_is_separate_from_code(self):
  cpu=self.execute(bytes([0x90,1,0,0x74,99,0xF0,0x22]))
  self.assertEqual(cpu.x[0x100],99);self.assertEqual(cpu.code[0x100],0x90)
 def test_unknown_opcode_and_rom_fail_closed(self):
  with self.assertRaisesRegex(AssertionError,'unsupported'):self.execute(bytes([0xA5]))
  with self.assertRaisesRegex(AssertionError,'Unmodeled ROM'):self.execute(bytes([0x12,0x12,0x34,0x22]))
 def test_backward_djnz_and_step_limit(self):
  cpu=self.execute(bytes([0x7F,3,0xDF,0xFE,0x22]));self.assertEqual(cpu.r(7),0)
  code=bytearray(65536);present=bytearray(65536);code[0x100:0x102]=bytes([0x80,0xFE]);present[0x100:0x102]=b'\x01\x01'
  with self.assertRaises(TimeoutError):Cpu(code,present,bytes(65536)).run(0x100,10)
 def test_multiply_flags_and_code_lookup(self):
  cpu=self.execute(bytes([0x74,255,0x75,0xF0,2,0xD3,0xA4,0x22]))
  self.assertEqual((cpu.a,cpu.d[0xF0],cpu.c,cpu.bit(0xD2)),(254,1,0,1))
  cpu=self.execute(bytes([0x90,1,7,0x74,1,0x93,0x22,42,99]))
  self.assertEqual(cpu.a,99)
 def test_rotate_through_carry(self):
  for value in [0,1,127,128,255]:
   for carry in [0,1]:
    cpu=self.execute(bytes([0x74,value,0xD3 if carry else 0xC3,0x33,0x22]))
    self.assertEqual((cpu.a,cpu.c),(((value<<1)|carry)&255,value>>7))
 def test_compare_incoming_borrow_and_inline_literal_store(self):
  cpu=self.execute(bytes([0xD3,0x12,0x3B,0x46,0x22]))
  self.assertEqual(cpu.c,1)
  cpu=self.execute(bytes([0x90,0x42,0,0x12,0x3A,0xD3,0xA5,2,3,4,0x74,42,0x22]))
  self.assertEqual(cpu.x[0x4200:0x4204],bytes([0xA5,2,3,4]));self.assertEqual(cpu.a,42)
if __name__=='__main__':unittest.main()
