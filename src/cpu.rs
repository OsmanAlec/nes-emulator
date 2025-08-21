use crate::opcodes;
use crate::opcodes::OpCode;
use crate::opcodes::CPU_OPS_CODES;
use crate::bus::Bus;
use std::collections::HashMap;


bitflags!{
    pub struct CpuFlags: u8 {
        const CARRY             = 0b00000001;
        const ZERO              = 0b00000010;
        const INTERRUPT_DISABLE = 0b00000100;
        const DECIMAL_MODE      = 0b00001000;
        const BREAK             = 0b00010000;
        const BREAK2            = 0b00100000;
        const OVERFLOW          = 0b01000000;
        const NEGATIV           = 0b10000000;
    }
}

const STACK: u16 = 0x0100;
const STACK_RESET: u8 = 0xfd;

pub struct CPU {
    pub register_a: u8,
    pub register_x: u8,
    pub register_p: CpuFlags,
    pub register_y: u8,
    pub program_counter: u16,
    pub stack_pointer: u8,
    pub bus: Bus,
}

#[derive(Debug)]
#[allow(non_camel_case_types)]
pub enum AddressingMode {
   Immediate,
   ZeroPage,
   ZeroPage_X,
   ZeroPage_Y,
   Absolute,
   Absolute_X,
   Absolute_Y,
   Indirect_X,
   Indirect_Y,
   NoneAddressing,
}

pub fn find_opcode(byte: u8) -> Option<&'static OpCode> {
    CPU_OPS_CODES.iter().find(|op| op.code == byte)
}

pub trait Mem {
    fn mem_read(&self, addr: u16) -> u8;

    fn mem_write(&mut self, addr: u16, data: u8);

    fn mem_read_u16(&self, pos: u16) -> u16 {
        let lo = self.mem_read(pos) as u16;
        let hi = self.mem_read(pos + 1) as u16;
        (hi << 8) | (lo as u16)
    }

    fn mem_write_u16(&mut self, pos: u16, data: u16) {
        let hi = (data >> 8) as u8;
        let lo = (data & 0xff) as u8;
        self.mem_write(pos, lo);
        self.mem_write(pos + 1, hi);
    }
}

impl Mem for CPU {
    fn mem_read(&self, addr: u16) -> u8 {
        self.bus.mem_read(addr)
    }

    fn mem_write(&mut self, addr: u16, data: u8) {
        self.bus.mem_write(addr, data)
    }

    fn mem_read_u16(&self, addr: u16) -> u16 {
        self.bus.mem_read_u16(addr)
    }

    fn mem_write_u16(&mut self, addr: u16, data: u16) {
        self.bus.mem_write_u16(addr, data)
    }
}

impl CPU {
    pub fn new(bus: Bus) -> Self {
        CPU {
            register_a: 0,
            register_x: 0,
            register_p: CpuFlags::from_bits_truncate(0b100100),
            register_y: 0,
            stack_pointer: STACK_RESET,
            program_counter: 0,
            bus: bus,
        }
    }

    fn stack_pop(&mut self) -> u8 {
        self.stack_pointer = self.stack_pointer.wrapping_add(1);
        self.mem_read((STACK as u16) + self.stack_pointer as u16)
    }

    fn stack_push(&mut self, data: u8) {
        self.mem_write((STACK as u16) + self.stack_pointer as u16, data);
        self.stack_pointer = self.stack_pointer.wrapping_sub(1)
    }

    fn stack_push_u16(&mut self, data: u16) {
        let hi = (data >> 8) as u8;
        let lo = (data & 0xff) as u8;
        self.stack_push(hi);
        self.stack_push(lo);
    }

    fn stack_pop_u16(&mut self) -> u16 {
        let lo = self.stack_pop() as u16;
        let hi = self.stack_pop() as u16;

        hi << 8 | lo
    }

    pub fn reset(&mut self) {
        self.register_a = 0;
        self.register_x = 0;
        self.register_y = 0;
        self.stack_pointer = STACK_RESET;
        self.register_p = CpuFlags::from_bits_truncate(0b100100);

        self.program_counter = self.mem_read_u16(0xFFFC);
    }

    pub fn load(&mut self, program: Vec<u8>) {
        for i in 0..(program.len() as u16) {
            self.mem_write(0x0600 + i, program[i as usize]);
        }
        //self.mem_write_u16(0xFFFC, 0x0000);
    }

    pub fn load_and_run(&mut self, program: Vec<u8>){
        self.load(program);
        self.program_counter = self.mem_read_u16(0xFFFC);
        self.run()
    }
    
    pub fn run(&mut self) {
        self.run_with_callback(|_| {});
    }

    pub fn run_with_callback<F>(&mut self, mut callback: F)
        where
            F: FnMut(&mut CPU),
        {
    
            let ref opcodes: HashMap<u8, &'static opcodes::OpCode> = *opcodes::OPCODES_MAP;

            loop {
                let code = self.mem_read(self.program_counter);
                self.program_counter += 1;
                let prev_program_counter = self.program_counter;

                let opcode = opcodes.get(&code).expect(&format!("OpCode {:x} is in the wrong format", code));
                
                match code {
                    /* LDA */
                    0xa9 | 0xa5 | 0xb5 | 0xad | 0xbd | 0xb9 | 0xa1 | 0xb1 => {
                        self.lda(&opcode.mode);
                    }
                    0x00 => return,

                    /* CLD */ 0xd8 => self.register_p.remove(CpuFlags::DECIMAL_MODE),

                    /* CLI */ 0x58 => self.register_p.remove(CpuFlags::INTERRUPT_DISABLE),

                    /* CLV */ 0xb8 => self.register_p.remove(CpuFlags::OVERFLOW),

                    /* CLC */ 0x18 => self.clear_carry_flag(),

                    /* SEC */ 0x38 => self.set_carry_flag(),

                    /* SEI */ 0x78 => self.register_p.insert(CpuFlags::INTERRUPT_DISABLE),

                    /* SED */ 0xf8 => self.register_p.insert(CpuFlags::DECIMAL_MODE),

                    /* PHA */ 0x48 => self.pha(),

                    /* PLA */
                    0x68 => {
                        self.pla();
                    }

                    /* PHP */
                    0x08 => {
                        self.php();
                    }

                    /* PLP */
                    0x28 => {
                        self.plp();
                    }

                    /* ADC */
                    0x69 | 0x65 | 0x75 | 0x6d | 0x7d | 0x79 | 0x61 | 0x71 => {
                        self.adc(&opcode.mode);
                    }

                    /* SBC */
                    0xe9 | 0xe5 | 0xf5 | 0xed | 0xfd | 0xf9 | 0xe1 | 0xf1 => {
                        self.sbc(&opcode.mode);
                    }

                    /* AND */
                    0x29 | 0x25 | 0x35 | 0x2d | 0x3d | 0x39 | 0x21 | 0x31 => {
                        self.and(&opcode.mode);
                    }

                    /* EOR */
                    0x49 | 0x45 | 0x55 | 0x4d | 0x5d | 0x59 | 0x41 | 0x51 => {
                        self.eor(&opcode.mode);
                    }

                    /* ORA */
                    0x09 | 0x05 | 0x15 | 0x0d | 0x1d | 0x19 | 0x01 | 0x11 => {
                        self.ora(&opcode.mode);
                    }

                    /* LSR */ 0x4a => self.lsr_register_a(),

                    /* LSR */
                    0x46 | 0x56 | 0x4e | 0x5e => {
                        self.lsr(&opcode.mode);
                    }

                    /*ASL*/ 0x0a => self.asl_register_a(),

                    /* ASL */
                    0x06 | 0x16 | 0x0e | 0x1e => {
                        self.asl(&opcode.mode);
                    }

                    /*ROL*/ 0x2a => self.rol_register_a(),

                    /* ROL */
                    0x26 | 0x36 | 0x2e | 0x3e => {
                        self.rol(&opcode.mode);
                    }

                    /* ROR */ 0x6a => self.ror_register_a(),

                    /* ROR */
                    0x66 | 0x76 | 0x6e | 0x7e => {
                        self.ror(&opcode.mode);
                    }

                    /* INC */
                    0xe6 | 0xf6 | 0xee | 0xfe => {
                        self.inc(&opcode.mode);
                    }

                    /* INX */
                    0xe8 => {
                        self.inx();
                    }

                    /* INY */
                    0xc8 => {
                        self.iny();
                    }

                    /* DEC */
                    0xc6 | 0xd6 | 0xce | 0xde => {
                        self.dec(&opcode.mode);
                    }

                    /* DEX */
                    0xca => {
                        self.dex();
                    }

                    /* DEY */
                    0x88 => {
                        self.dey();
                    }

                    /* CMP */
                    0xc9 | 0xc5 | 0xd5 | 0xcd | 0xdd | 0xd9 | 0xc1 | 0xd1 => {
                        self.compare(&opcode.mode, self.register_a);
                    }

                    /* CPY */
                    0xc0 | 0xc4 | 0xcc => {
                        self.compare(&opcode.mode, self.register_y);
                    }

                    /* CPX */
                    0xe0 | 0xe4 | 0xec => self.compare(&opcode.mode, self.register_x),

                    /* JMP Absolute */
                    0x4c => {
                        let mem_address = self.mem_read_u16(self.program_counter);
                        self.program_counter = mem_address;
                    }

                    /* JMP Indirect */
                    0x6c => {
                        let mem_address = self.mem_read_u16(self.program_counter);
                
                        let indirect_ref = if mem_address & 0x00FF == 0x00FF {
                            let lo = self.mem_read(mem_address);
                            let hi = self.mem_read(mem_address & 0xFF00);
                            (hi as u16) << 8 | (lo as u16)
                        } else {
                            self.mem_read_u16(mem_address)
                        };

                        self.program_counter = indirect_ref;
                    }

                    /* JSR */
                    0x20 => {
                        self.stack_push_u16(self.program_counter + 2 - 1);
                        let target_address = self.mem_read_u16(self.program_counter);
                        self.program_counter = target_address
                    }

                    /* RTS */
                    0x60 => {
                        self.program_counter = self.stack_pop_u16() + 1;
                    }

                    /* RTI */
                    0x40 => {
                        self.register_p.bits = self.stack_pop();
                        self.register_p.remove(CpuFlags::BREAK);
                        self.register_p.insert(CpuFlags::BREAK2);

                        self.program_counter = self.stack_pop_u16();
                    }

                    /* BNE */
                    0xd0 => {
                        self.branch(!self.register_p.contains(CpuFlags::ZERO));
                    }

                    /* BVS */
                    0x70 => {
                        self.branch(self.register_p.contains(CpuFlags::OVERFLOW));
                    }

                    /* BVC */
                    0x50 => {
                        self.branch(!self.register_p.contains(CpuFlags::OVERFLOW));
                    }

                    /* BPL */
                    0x10 => {
                        self.branch(!self.register_p.contains(CpuFlags::NEGATIV));
                    }

                    /* BMI */
                    0x30 => {
                        self.branch(self.register_p.contains(CpuFlags::NEGATIV));
                    }

                    /* BEQ */
                    0xf0 => {
                        self.branch(self.register_p.contains(CpuFlags::ZERO));
                    }

                    /* BCS */
                    0xb0 => {
                        self.branch(self.register_p.contains(CpuFlags::CARRY));
                    }

                    /* BCC */
                    0x90 => {
                        self.branch(!self.register_p.contains(CpuFlags::CARRY));
                    }

                    /* BIT */
                    0x24 | 0x2c => {
                        self.bit(&opcode.mode);
                    }

                    /* STA */
                    0x85 | 0x95 | 0x8d | 0x9d | 0x99 | 0x81 | 0x91 => {
                        self.sta(&opcode.mode);
                    }

                    /* STX */
                    0x86 | 0x96 | 0x8e => {
                        self.stx(&opcode.mode);
                    }

                    /* STY */
                    0x84 | 0x94 | 0x8c => {
                        self.sty(&opcode.mode);
                    }

                    /* LDX */
                    0xa2 | 0xa6 | 0xb6 | 0xae | 0xbe => {
                        self.ldx(&opcode.mode);
                    }

                    /* LDY */
                    0xa0 | 0xa4 | 0xb4 | 0xac | 0xbc => {
                        self.ldy(&opcode.mode);
                    }

                    /* NOP */
                    0xea => {
                        //do nothing
                    }

                    0xaa => {
                        self.tax();
                    }

                    /* TAY */
                    0xa8 => {
                        self.tay();
                    }

                    /* TSX */
                    0xba => {
                        self.tsx();
                    }

                    /* TXA */
                    0x8a => {
                        self.txa();
                    }

                    /* TXS */
                    0x9a => {
                        self.txs();
                    }

                    /* TYA */
                    0x98 => {
                        self.tya();
                    }

                    _ => todo!(),
                }
                if prev_program_counter == self.program_counter{
                    self.program_counter += (opcode.bytes-1) as u16;
                }

                callback(self);
            }
        }  
    


    fn tax(&mut self) {
        self.register_x = self.register_a;
        self.update_zero_and_negative_flags(self.register_x);
    }

    fn tay(&mut self) {
        self.register_y = self.register_a;
        self.update_zero_and_negative_flags(self.register_y);
    }

    fn txa(&mut self) {
        self.register_a = self.register_x;
        self.update_zero_and_negative_flags(self.register_a);
    }

    fn tya(&mut self) {
        self.register_a = self.register_y;
        self.update_zero_and_negative_flags(self.register_a);
    }

    fn tsx(&mut self) {
        self.register_x = self.stack_pointer;
        self.update_zero_and_negative_flags(self.register_x);
    }


    fn txs(&mut self) {
        self.stack_pointer = self.register_x;
    }

    fn inc(&mut self, mode: &AddressingMode) -> u8{
        let addr = self.get_operand_address(mode);
        let mut val = self.mem_read(addr);
        val = val.wrapping_add(1);
        self.mem_write(addr, val);
        self.update_zero_and_negative_flags(val);
        val
    }

    fn adc(&mut self, mode: &AddressingMode){
        let addr = self.get_operand_address(mode);
        let val = self.mem_read(addr);
        self.add_to_register_a(val);
    }

    fn and(&mut self, mode: &AddressingMode){
        let addr = self.get_operand_address(mode);
        let val = self.mem_read(addr);
        self.set_register_a(val & self.register_a);
    }

    fn asl_register_a (&mut self){
        let mut var = self.register_a;
        if self.register_a >> 7 == 1 {
            self.set_carry_flag();
        } else {
            self.clear_carry_flag();
        }
        var <<= 1;
        self.set_register_a(var);
    }

    fn asl(&mut self, mode: &AddressingMode) -> u8{
        let addr = self.get_operand_address(mode);
        let mut val = self.mem_read(addr);

        if val >> 7 == 1 {
            self.set_carry_flag();
        } else {
            self.clear_carry_flag();
        }
        
        val = val << 1;
        self.mem_write(addr, val);
        self.update_zero_and_negative_flags(val);
        val
    }

    fn bit(&mut self, mode: &AddressingMode){
        let addr = self.get_operand_address(mode);
        let val = self.mem_read(addr);

        let and = self.register_a & val;
        if and == 0 {
            self.register_p.insert(CpuFlags::ZERO);
        } else {
            self.register_p.remove(CpuFlags::ZERO);
        }

        self.register_p.set(CpuFlags::NEGATIV, val & 0b1000_0000 > 0);
        self.register_p.set(CpuFlags::OVERFLOW, val & 0b0100_0000 > 0);
    }

    fn dec(&mut self, mode: &AddressingMode) -> u8{
        let addr = self.get_operand_address(mode);
        let val = self.mem_read(addr);
        let result = val.wrapping_sub(1);
        self.mem_write(addr, result);
        self.update_zero_and_negative_flags(result);
        result
    }

    fn eor(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        let var = self.mem_read(addr);
        self.register_a ^= var;
        self.update_zero_and_negative_flags(self.register_a);
    }

    fn jsr(&mut self){
        self.stack_push_u16(self.program_counter + 2 - 1);
        let target_address = self.mem_read_u16(self.program_counter);
        self.program_counter = target_address
    }

    fn lsr_register_a(&mut self){
        let mut var = self.register_a;
        if self.register_a & 1 == 1 {
            self.set_carry_flag();
        } else {
            self.clear_carry_flag();
        }
        var >>= 1;
        self.set_register_a(var);
    }

    fn lsr(&mut self, mode: &AddressingMode) -> u8{
        let addr = self.get_operand_address(mode);
        let mut var = self.mem_read(addr);

        if var & 1 == 1 {
            self.set_carry_flag();
        }

        var >>= 1;
        self.mem_write(addr, var);
        self.update_zero_and_negative_flags(var);
        var
    }

    fn ora(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        let mut var = self.mem_read(addr);
        var |= self.register_a;
        self.set_register_a(var);
    }

    fn pha(&mut self){
        let var = self.register_a;
        self.stack_push(var);
    }

    fn php(&mut self){
        let mut flags = self.register_p.clone();
        flags.insert(CpuFlags::BREAK);
        flags.insert(CpuFlags::BREAK2);
        self.stack_push(flags.bits());
    }

    fn pla(&mut self){
        let var = self.stack_pop();
        self.set_register_a(var);
    }

    fn plp(&mut self){
        self.register_p.bits = self.stack_pop();
        self.register_p.remove(CpuFlags::BREAK);
        self.register_p.remove(CpuFlags::BREAK2);
    }

    fn rol_register_a(&mut self){
        let old_carry = self.register_p.contains(CpuFlags::CARRY);

        if self.register_a >> 7 == 1{
            self.set_carry_flag();
        } else {
            self.clear_carry_flag();
        }
        self.register_a <<= 1;

        if old_carry {
            self.register_a |= 1;
        }
        self.update_zero_and_negative_flags(self.register_a);
    }

    fn rol(&mut self, mode: &AddressingMode) -> u8 {
        let addr = self.get_operand_address(mode);
        let mut var = self.mem_read(addr);
        let old_carry = self.register_p.contains(CpuFlags::CARRY);

        if var >> 7 == 1 {
            self.set_carry_flag();
        } else {
            self.clear_carry_flag();
        }
        var = var << 1;
        if old_carry {
            var = var | 1;
        }
        self.mem_write(addr, var);
        self.update_zero_and_negative_flags(var);
        var
    }

    fn ror(&mut self, mode: &AddressingMode) -> u8 {
        let addr = self.get_operand_address(mode);
        let mut var = self.mem_read(addr);
        let old_carry = self.register_p.contains(CpuFlags::CARRY);

        if var & 1 == 1 {
            self.set_carry_flag();
        } else {
            self.clear_carry_flag();
        }
        var = var >> 1;
        if old_carry {
            var = var | 0b10000000;
        }
        self.mem_write(addr, var);
        self.update_zero_and_negative_flags(var);
        var
    }

    fn ror_register_a(&mut self) {
        let old_carry = self.register_p.contains(CpuFlags::CARRY);

        if self.register_a & 1 == 1 {
            self.set_carry_flag();
        } else {
            self.clear_carry_flag();
        }
        self.register_a >>= 1;
        if old_carry {
            self.register_a |= 0b10000000;
        }
        self.update_zero_and_negative_flags(self.register_a);
    }

    fn rti(&mut self){
        self.register_p.bits = self.stack_pop();
        self.register_p.remove(CpuFlags::BREAK);
        self.register_p.insert(CpuFlags::BREAK2);
        self.program_counter = self.stack_pop_u16();
    }

    fn sbc(&mut self, mode: &AddressingMode){
        let addr = self.get_operand_address(mode);
        let var = self.mem_read(addr);

        self.add_to_register_a(((var as i8).wrapping_neg().wrapping_sub(1)) as u8);
    }

    fn sta(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        self.mem_write(addr, self.register_a);
    }

    fn stx(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        self.mem_write(addr, self.register_x);
    }

    fn sty(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        self.mem_write(addr, self.register_y);
    }
    
    fn inx(&mut self) {
        self.register_x = self.register_x.wrapping_add(1);
        self.update_zero_and_negative_flags(self.register_x);
    }

    fn iny(&mut self) {
        self.register_y = self.register_y.wrapping_add(1);
        self.update_zero_and_negative_flags(self.register_y);
    }

    fn dex(&mut self) {
        self.register_x = self.register_x.wrapping_sub(1);
        self.update_zero_and_negative_flags(self.register_x);
    }

    fn dey(&mut self) {
        self.register_y = self.register_y.wrapping_sub(1);
        self.update_zero_and_negative_flags(self.register_y);
    }

    fn compare(&mut self, mode: &AddressingMode, compare_with: u8){
        let addr = self.get_operand_address(mode);
        let val = self.mem_read(addr);

        if compare_with >= val {
            self.register_p.insert(CpuFlags::CARRY);
        } else {
            self.register_p.remove(CpuFlags::CARRY);
        }

        self.update_zero_and_negative_flags(compare_with.wrapping_sub(val));
    }

    fn lda(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        let var = self.mem_read(addr);
        self.set_register_a(var);
    }

    fn ldx(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        let var = self.mem_read(addr);
        self.register_x = var;
        self.update_zero_and_negative_flags(self.register_x);
    }

    fn ldy(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        let var = self.mem_read(addr);
        self.register_y = var;
        self.update_zero_and_negative_flags(self.register_y);
    }
    
    fn branch(&mut self, condition:bool){
        if condition {
            let jump: i8 = self.mem_read(self.program_counter) as i8;
            let jump_addr = self
                .program_counter
                .wrapping_add(1)
                .wrapping_add(jump as u16);

            self.program_counter = jump_addr;
        }
    }

    fn set_carry_flag(&mut self){
        self.register_p.insert(CpuFlags::CARRY)
    }

    fn clear_carry_flag(&mut self){
        self.register_p.remove(CpuFlags::CARRY)
    }

    fn update_zero_and_negative_flags(&mut self, result: u8){
        if result == 0 {
            self.register_p.insert(CpuFlags::ZERO);
        } else {
            self.register_p.remove(CpuFlags::ZERO);
        }

        if result & 0b1000_0000 != 0 {
            self.register_p.insert(CpuFlags::NEGATIV);
        } else {
            self.register_p.remove(CpuFlags::NEGATIV);
        }
    }

    fn set_register_a(&mut self, val: u8){
        self.register_a = val;
        self.update_zero_and_negative_flags(self.register_a);
    }

    fn add_to_register_a(&mut self, data: u8){
        let sum = self.register_a as u16 
        + data as u16 
        + (if self.register_p.contains(CpuFlags::CARRY){
            1
        } else {0}) as u16;

        let carry = sum > 0xff;
        if carry {
            self.set_carry_flag();
        } else {
            self.clear_carry_flag();
        }

        let result = sum as u8;

        if (data ^ result) & (result ^ self.register_a) & 0x80 != 0 {
            self.register_p.insert(CpuFlags::OVERFLOW);
        } else {
            self.register_p.remove(CpuFlags::OVERFLOW);
        }

        self.set_register_a(result);
    }

    fn get_operand_address(&self, mode: &AddressingMode) -> u16 {

       match mode {
           AddressingMode::Immediate => self.program_counter,

           AddressingMode::ZeroPage  => self.mem_read(self.program_counter) as u16,
          
           AddressingMode::Absolute => self.mem_read_u16(self.program_counter),
        
           AddressingMode::ZeroPage_X => {
               let pos = self.mem_read(self.program_counter);
               let addr = pos.wrapping_add(self.register_x) as u16;
               addr
           }
           AddressingMode::ZeroPage_Y => {
               let pos = self.mem_read(self.program_counter);
               let addr = pos.wrapping_add(self.register_y) as u16;
               addr
           }

           AddressingMode::Absolute_X => {
               let base = self.mem_read_u16(self.program_counter);
               let addr = base.wrapping_add(self.register_x as u16);
               addr
           }
           AddressingMode::Absolute_Y => {
               let base = self.mem_read_u16(self.program_counter);
               let addr = base.wrapping_add(self.register_y as u16);
               addr
           }

           AddressingMode::Indirect_X => {
               let base = self.mem_read(self.program_counter);

               let ptr: u8 = (base as u8).wrapping_add(self.register_x);
               let lo = self.mem_read(ptr as u16);
               let hi = self.mem_read(ptr.wrapping_add(1) as u16);
               (hi as u16) << 8 | (lo as u16)
           }
           AddressingMode::Indirect_Y => {
               let base = self.mem_read(self.program_counter);

               let lo = self.mem_read(base as u16);
               let hi = self.mem_read((base as u8).wrapping_add(1) as u16);
               let deref_base = (hi as u16) << 8 | (lo as u16);
               let deref = deref_base.wrapping_add(self.register_y as u16);
               deref
           }
         
           AddressingMode::NoneAddressing => {
               panic!("mode {:?} is not supported", mode);
           }
        }
    }
}


#[cfg(test)]
mod test {
    use super::*;
    use crate::cartridge::test;

    #[test]
    fn test_0xa9_lda_immediate_load_data() {
        let bus = Bus::new(test::test_rom(vec![0xa9, 0x05, 0x00]));
        let mut cpu = CPU::new(bus);

        cpu.run();

        assert_eq!(cpu.register_a, 5);
        assert!(cpu.register_p.bits() & 0b0000_0010 == 0b00);
        assert!(cpu.register_p.bits() & 0b1000_0000 == 0);
    }

    #[test]
    fn test_0xaa_tax_move_a_to_x() {
        let bus = Bus::new(test::test_rom(vec![0xaa, 0x00]));
        let mut cpu = CPU::new(bus);
        cpu.register_a = 10;

        cpu.run();

        assert_eq!(cpu.register_x, 10)
    }

    #[test]
    fn test_5_ops_working_together() {
        let bus = Bus::new(test::test_rom(vec![0xa9, 0xc0, 0xaa, 0xe8, 0x00]));
        let mut cpu = CPU::new(bus);

        cpu.run();

        assert_eq!(cpu.register_x, 0xc1)
    }

    #[test]
    fn test_inx_overflow() {
        let bus = Bus::new(test::test_rom(vec![0xe8, 0xe8, 0x00]));
        let mut cpu = CPU::new(bus);
        cpu.register_x = 0xff;

        cpu.run();

        assert_eq!(cpu.register_x, 1)
    }

    #[test]
    fn test_lda_from_memory() {
        let bus = Bus::new(test::test_rom(vec![0xa5, 0x10, 0x00]));
        let mut cpu = CPU::new(bus);
        cpu.mem_write(0x10, 0x55);

        cpu.run();

        assert_eq!(cpu.register_a, 0x55);
    }
}