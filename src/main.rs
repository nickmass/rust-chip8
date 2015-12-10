extern crate rustc_serialize;
extern crate docopt;

use docopt::Docopt;

const USAGE: &'static str = "
rust-chip8

Usage:
    rust-chip8 <file>
    rust-chip8 (-h | --help)

Options:
    -h --help   Show this screen
";


#[derive(Debug, RustcDecodable)]
struct Args {
    arg_file: String,
}

fn main() {
    let args: Args = Docopt::new(USAGE)
        .and_then(|d| d.decode())
        .unwrap_or_else(|e| e.exit());

    let mut cpu = Cpu::new();
    cpu.run();
}

struct Cpu {
    disp: Display,
    mem: Memory,
    regs: Registers,
}

impl Cpu {
    fn new() -> Cpu {
        Cpu {
            disp: Display::new(),
            mem: Memory::new(),
            regs: Registers::new(),
        }
    }

    fn run(&mut self) {
        loop {
            let opcode  = self.read_opcode();
            match opcode {
                (0, 0, 0xE, 0) => self.clear_screen(), //Clear screen
                (0, 0, 0xE, 0xE) => self.ret(), //ret
                (0, _, _, _) => {}, //RCA program
                (1, a, b, c) => self.jump(Self::join_three(a,b,c)),
                (2, a, b, c) => self.sub(Self::join_three(a,b,c)),
                (3, a, b, c) => self.skip_if(a, Self::join_two(b,c)),
                (4, a, b, c) => self.skip_if_not(a, Self::join_two(b,c)),
                (5, a, b, 0) => self.skip_if_reg(a, b),
                (6, a, b, c) => self.set(a, Self::join_two(b,c)),
                (7, a, b, c) => self.add(a, Self::join_two(b,c)),
                (8, a, b, 0) => self.set_reg(a, b),
                (8, a, b, 1) => self.or_reg(a, b),
                (8, a, b, 2) => self.and_reg(a,b),
                (8, a, b, 3) => self.xor_reg(a,b),
                (8, a, b, 4) => self.add_reg(a,b),
                (8, a, b, 5) => self.cmp_reg(a,b),
                (8, a, b, 6) => self.shift_right_reg(a,b),
                (8, a, b, 7) => self.sub_reg(a,b),
                (8, a, b, 0xE) => self.shift_left_reg(a,b),
                (9, a, b, 0) => self.skip_if_not_reg(a,b),
                (0xA, a, b, c) => self.set_index(Self::join_three(a,b,c)),
                (0xB, a, b, c) => self.jump_offset(Self::join_three(a,b,c)),
                (0xC, a, b, c) => self.random(a, Self::join_two(b,c)),
                (0xD, a, b, c) => self.draw_sprite(a,b,c),
                (0xE, a, 9, 0xE) => self.skip_if_key(a),
                (0xE, a, 0xA, 1) => self.skip_if_not_key(a),
                (0xF, a, 0, 7) => self.set_from_delay_timer(a),
                (0xF, a, 0, 0xA) => self.wait_for_key(a),
                (0xF, a, 1, 5) => self.set_delay_timer(a),
                (0xF, a, 1, 8) => self.set_sound_timer(a),
                (0xF, a, 1, 0xE) => self.add_to_index(a),
                (0xF, a, 2, 9) => self.set_index_to_character(a),
                (0xF, a, 3, 3) => self.store_bcd(a),
                (0xF, a, 5, 5) => self.store_to_index(a),
                (0xF, a, 6, 5) => self.fill_from_index(a),
                _ => {},
            };
        }
    }

    fn clear_screen(&mut self) {
        self.disp.clear_screen();
    }

    fn skip_if(&mut self, reg: u8, value: u8) {
        if self.regs.data[reg as usize] == value {
            self.regs.address = self.regs.address + 2;
        }
    }

    fn skip_if_not(&mut self, reg: u8, value: u8) {
        if self.regs.data[reg as usize] != value {
            self.regs.address = self.regs.address + 2;
        }
    }

    fn skip_if_reg(&mut self, reg_a: u8, reg_b: u8) {
        if self.regs.data[reg_a as usize] == self.regs.data[reg_b as usize] {
            self.regs.address = self.regs.address + 2;
        }
    }

    fn set(&mut self, reg: u8, value: u8) {
        self.regs.data[reg as usize] = value;
    }

    fn add(&mut self, reg: u8, value: u8) {
        self.regs.data[reg as usize] = self.regs.data[reg as usize] + value;
    }

    fn set_reg(&mut self, reg_a: u8, reg_b: u8) {
        self.regs.data[reg_a as usize] = self.regs.data[reg_b as usize];
    }

    fn or_reg(&mut self, reg_a: u8, reg_b: u8) {
        self.regs.data[reg_a as usize] = self.regs.data[reg_a as usize] | self.regs.data[reg_b as usize];
    }

    fn and_reg(&mut self, reg_a: u8, reg_b: u8) {
        self.regs.data[reg_a as usize] = self.regs.data[reg_a as usize] & self.regs.data[reg_b as usize];
    }

    fn xor_reg(&mut self, reg_a: u8, reg_b: u8) {
        self.regs.data[reg_a as usize] = self.regs.data[reg_a as usize] ^ self.regs.data[reg_b as usize];
    }

    fn add_reg(&mut self, reg_a: u8, reg_b: u8) {
        if (reg_a as u16) + (reg_b as u16) > 255 {
            self.regs.data[0xF] = 1;
        } else {
            self.regs.data[0xF] = 0;
        }
    }

    fn cmp_reg(&mut self, reg_a: u8, reg_b: u8) {
        if (reg_a as i16) - (reg_b as i16) < 0 {
            self.regs.data[0xF] = 1;
        } else {
            self.regs.data[0xF] = 0;
        }
    }

    fn shift_right_reg(&mut self, reg_a: u8, reg_b: u8) {
        self.regs.data[0xF] =  self.regs.data[reg_a as usize] & 1;
        self.regs.data[reg_a as usize] = self.regs.data[reg_a as usize] >> 1;
    }

    fn sub_reg(&mut self, reg_a: u8, reg_b: u8) {
        
    }

    fn shift_left_reg(&mut self, reg_a: u8, reg_b: u8) {
        let carry = self.regs.data[reg_a as usize] & 0x80;
        if carry != 0 {
            self.regs.data[0xF] = 1;
        } else {
            self.regs.data[0xF] = 0;
        }

        self.regs.data[reg_a as usize] = self.regs.data[reg_a as usize] << 1;
    }

    fn skip_if_not_reg(&mut self, reg_a: u8, reg_b: u8) {
        if self.regs.data[reg_a as usize] != self.regs.data[reg_b as usize] {
            self.regs.address = self.regs.address + 2;
        }
    }

    fn set_index(&mut self, value: u16) {
        self.regs.index = value;
    }

    fn jump_offset(&mut self, addr: u16) {
        self.regs.address = self.regs.data[0] as u16 + addr;
    }

    fn random(&mut self, reg: u8, value: u8) {
        self.regs.data[reg as usize] = 4 ^ value;
    }

    fn draw_sprite(&mut self, reg_a: u8, reg_b: u8, rows: u8) {
        for n in 0..rows {
            self.disp.draw_line(self.mem.read(self.regs.index + n as u16), self.regs.data[reg_a as usize], self.regs.data[(reg_b + n) as usize])
        }
    }

    fn skip_if_key(&mut self, reg: u8) {
    }

    fn skip_if_not_key(&mut self, reg: u8) {
    }

    fn set_from_delay_timer(&mut self, reg: u8) {
    }

    fn wait_for_key(&mut self, reg: u8) {
    }

    fn set_delay_timer(&mut self, reg: u8) {
    }

    fn set_sound_timer(&mut self, reg: u8) {
    }

    fn add_to_index(&mut self, reg: u8) {
        self.regs.index = self.regs.index + self.regs.data[reg as usize] as u16;
    }

    fn set_index_to_character(&mut self, reg: u8) {
    }

    fn store_bcd(&mut self, reg: u8) {
    }

    fn store_to_index(&mut self, reg: u8) {
        for n in 0..reg {
            self.mem.write(self.regs.index + n as u16, self.regs.data[(reg + n) as usize]);
        }
    }

    fn fill_from_index(&mut self, reg: u8) {
        for n in 0..reg {
            self.regs.data[(reg + n) as usize] = self.mem.read(self.regs.index + n as u16);
        }
    }

    fn read_opcode(&mut self) -> (u8, u8, u8, u8) {
        let word = self.mem.read_word(self.regs.address);
        self.regs.address = self.regs.address + 2;
        ((word & 0xF0 >> 4) as u8, (word & 0xF) as u8, (word & 0xF000 >> 12) as u8, (word & 0xF00 >> 8) as u8)
    }
    
    fn jump(&mut self, address: u16) {
        self.regs.address = address;
    }

    fn sub(&mut self, address: u16) {
        let return_addr = self.regs.address;
        self.push_addr(return_addr);
        self.regs.address = address;
    }

    fn ret(&mut self) {
        self.regs.address = self.pop_addr();
    }

    fn push(&mut self, value: u8) {
        self.mem.write(self.regs.stack, value);
        self.regs.stack = self.regs.stack + 1;
    }

    fn push_addr(&mut self, address: u16) {
       let x = (address & 0xFF) as u8;
       let y = ((address >> 8) & 0xFF) as u8;
       self.push(x);
       self.push(y);
    }

    fn pop(&mut self) -> u8 {
        self.regs.stack = self.regs.stack - 1;
        self.mem.read(self.regs.stack)
    }

    fn pop_addr(&mut self) -> u16 {
        let x = self.pop() as u16;
        let y = self.pop() as u16;

        x | (y << 8)
    }
    
    fn join_two(a: u8, b: u8) -> u8 {
        let a1 = a as u8;
        let b1 = b as u8;
        (((a1 & 0xF) << 4) | (b1 & 0xF)) as u8
    }

    fn join_three(a: u8, b: u8, c: u8) -> u16 {
        let a1 = a as u16;
        let b1 = b as u16;
        let c1 = c as u16;
        (((a1 & 0xF) << 8) | ((b1 & 0xF) << 4) | (c1 & 0xF)) as u16
    }
}

struct Display {
    screen: [u8;2048],
}

impl Display {
    fn new() -> Display {
        Display {
            screen: [0; 2048]
        }
    }

    fn clear_screen(&mut self) {
        for n in 0..2048 {
            self.screen[n] = 0;
        }
    }

    fn toggle_pixel(&mut self, pixel: u8,  x: u8, y: u8) {
        let real_x = x & 0x3F;
        let real_y = y & 0x1F;
        let offset = ((real_y * 64) + real_x) as usize;
        self.screen[offset] = pixel ^ self.screen[offset];
    }

    fn draw_line(&mut self, line: u8, x: u8, y: u8) {
        for n in 0..8 {
            self.toggle_pixel((line >> n) & 1, x + n, y);
        }
    }
}

struct Registers {
    data: [u8;16],
    address: u16,
    stack: u16,
    index: u16,
}

impl Registers { 
    fn new() -> Registers {
        Registers {
            data: [0; 16],
            address: 0,
            stack: 0xEA0,
            index: 0,
        }
    }
}

struct Memory {
    bytes: [u8;0x4000],
}

impl Memory {
    fn new() -> Memory {
        Memory {
            bytes: [1; 0x4000]
        }
    }

    fn read(&self, addr: u16) -> u8 {
        let safe_addr = addr & 0xFFF;
        self.bytes[safe_addr as usize]
    }

    fn read_word(&self, addr: u16) -> u16 {
        (self.read(addr) as u16) | ((self.read(addr + 1) as u16) << 8)
    }

    fn write(&mut self, addr: u16, value: u8) {
        let safe_addr = addr & 0xFFF;
        self.bytes[addr as usize] = value;
    }

    fn write_word(&mut self, addr: u16, value: u16) {
        self.write(addr, (value & 0xFF) as u8);
        self.write(addr + 1, ((value >> 8) & 0xFF) as u8);
    }
}
