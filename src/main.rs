extern crate rustc_serialize;
extern crate docopt;
extern crate rand;

#[macro_use]
extern crate glium;

mod traits;
use self::traits::*;

use docopt::Docopt;
use std::fs::File;
use std::io::{self, Write, Read};

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

    let f = File::open(args.arg_file).unwrap();
    let mut rom: Vec<u8> = Vec::new();
    let _ = f.take(0x1000 - 0x200).read_to_end(&mut rom).unwrap();
    let mut cpu = Cpu::<GliumRenderer>::new(rom);
    loop {
        if cpu.run() { break; }
    }
}

struct Cpu<T: Chip8System> {
    disp: Display,
    mem: Memory,
    regs: Registers,
    system: T,
    wait_on_input: Option<u8>,
}

mod chip_gl;
use self::chip_gl::*;

impl<T: Chip8System> Cpu<T> {
    fn new(rom: Vec<u8>) -> Cpu<GliumRenderer> {
        Cpu {
            disp: Display::new(),
            mem: Memory::new_with_rom(rom),
            regs: Registers::new(),
            system: GliumRenderer::new(),
            wait_on_input: None,
        }
    }

    fn run(&mut self) -> bool {
        let mut draw_countdown = 60000; //No idea
        loop {
            if let Some(reg) = self.wait_on_input {
                if let Some(key) = self.system.get_input() {
                    self.regs.set_data(reg, key);
                    self.wait_on_input = None;
                }
            } else {
                let opcode  = self.read_opcode();
                match opcode {
                    (0, 0, 0xE, 0) => self.clear_screen(),
                    (0, 0, 0xE, 0xE) => self.ret(),
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
            draw_countdown -= 1;
            if draw_countdown == 0 {
                if self.regs.delay_timer != 0 { self.regs.delay_timer -= 1; }
                if self.regs.sound_timer != 0 { self.regs.sound_timer -= 1; }
                self.system.render(&self.disp.screen);
                break;
            }
        }

        self.system.is_closed()
    }

    fn clear_screen(&mut self) {
        self.disp.clear_screen();
    }

    fn skip_if(&mut self, reg: u8, value: u8) {
        if self.regs.get_data(reg) == value {
            self.regs.address = self.regs.address.wrapping_add(2);
        }
    }

    fn skip_if_not(&mut self, reg: u8, value: u8) {
        if self.regs.get_data(reg) != value {
            self.regs.address = self.regs.address.wrapping_add(2);
        }
    }

    fn skip_if_reg(&mut self, reg_a: u8, reg_b: u8) {
        if self.regs.get_data(reg_a) == self.regs.get_data(reg_b) {
            self.regs.address = self.regs.address.wrapping_add(2);
        }
    }

    fn set(&mut self, reg: u8, value: u8) {
        self.regs.set_data(reg, value);
    }

    fn add(&mut self, reg: u8, value: u8) {
        let reg_val = self.regs.get_data(reg);
        self.regs.set_data(reg, reg_val.wrapping_add(value));
    }

    fn set_reg(&mut self, reg_a: u8, reg_b: u8) {
        let val = self.regs.get_data(reg_b);
        self.regs.set_data(reg_a, val);
    }

    fn or_reg(&mut self, reg_a: u8, reg_b: u8) {
        let val_left = self.regs.get_data(reg_a);
        let val_right = self.regs.get_data(reg_b);
        self.regs.set_data(reg_a, val_left | val_right);
    }

    fn and_reg(&mut self, reg_a: u8, reg_b: u8) {
        let val_left = self.regs.get_data(reg_a);
        let val_right = self.regs.get_data(reg_b);
        self.regs.set_data(reg_a, val_left & val_right);
    }

    fn xor_reg(&mut self, reg_a: u8, reg_b: u8) {
        let val_left = self.regs.get_data(reg_a);
        let val_right = self.regs.get_data(reg_b);
        self.regs.set_data(reg_a, val_left ^ val_right);
    }

    fn add_reg(&mut self, reg_a: u8, reg_b: u8) {                
        let val_left = self.regs.get_data(reg_b);
        let val_right = self.regs.get_data(reg_a);
        if (val_left as u16) + (val_right as u16) > 255 {
            self.regs.set_data(0xF, 1);
        } else {
            self.regs.set_data(0xF, 0);
        }
        
        self.regs.set_data(reg_a, val_left.wrapping_add(val_right));
    }

    fn cmp_reg(&mut self, reg_a: u8, reg_b: u8) {
        let val_left = self.regs.get_data(reg_a);
        let val_right = self.regs.get_data(reg_b);
        if val_left > val_right {
            self.regs.set_data(0xF, 1);
        } else {
            self.regs.set_data(0xF, 0);
        }

        self.regs.set_data(reg_a, val_left.wrapping_sub(val_right));
    }

    fn shift_right_reg(&mut self, reg_a: u8, _: u8) {
        let val = self.regs.get_data(reg_a);
        self.regs.set_data(0xF, val & 1);
        self.regs.set_data(reg_a, val.wrapping_shr(1));
    }

    fn sub_reg(&mut self, reg_a: u8, reg_b: u8) {
        let val_left = self.regs.get_data(reg_b);
        let val_right = self.regs.get_data(reg_a);
        if val_left > val_right {
            self.regs.set_data(0xF, 1);
        } else {
            self.regs.set_data(0xF, 0);
        }

        self.regs.set_data(reg_a, val_left.wrapping_sub(val_right));
    }

    fn shift_left_reg(&mut self, reg_a: u8, _: u8) {
        let val = self.regs.get_data(reg_a);
        let carry = val & 0x80;
        if carry != 0 {
            self.regs.set_data(0xF, 1);
        } else {
            self.regs.set_data(0xF, 0);
        }

        self.regs.set_data(reg_a, val.wrapping_shl(1));
    }

    fn skip_if_not_reg(&mut self, reg_a: u8, reg_b: u8) {
        if self.regs.get_data(reg_a) != self.regs.get_data(reg_b) {
            self.regs.address = self.regs.address.wrapping_add(2);
        }
    }

    fn set_index(&mut self, value: u16) {
        self.regs.index = value;
    }

    fn jump_offset(&mut self, addr: u16) {
        self.regs.address = (self.regs.get_data(0) as u16).wrapping_add(addr);
    }

    fn random(&mut self, reg: u8, value: u8) {
        self.regs.set_data(reg, rand::random::<u8>() & value);
    }

    fn draw_sprite(&mut self, reg_a: u8, reg_b: u8, rows: u8) {
        let mut flipped = false;
        for n in 0..rows {
            flipped |= self.disp.draw_line(self.mem.read(self.regs.index.wrapping_add(n as u16)), self.regs.get_data(reg_a), self.regs.get_data(reg_b).wrapping_add(n));
        }

        self.regs.set_data(0xF, if flipped { 1 } else { 0 });
    }

    fn skip_if_key(&mut self, reg: u8) {
        if let Some(key) = self.system.get_input() {
            if key == self.regs.get_data(reg) { 
                self.regs.address = self.regs.address.wrapping_add(2);
            }
        }
    }

    fn skip_if_not_key(&mut self, reg: u8) {
        if let Some(key) = self.system.get_input() {
            if key == self.regs.get_data(reg) { 
                return;
            }
        }

        self.regs.address = self.regs.address.wrapping_add(2);
    }

    fn set_from_delay_timer(&mut self, reg: u8) {
        let val = self.regs.delay_timer;
        self.regs.set_data(reg, val);
    }

    fn wait_for_key(&mut self, reg: u8) {
        self.wait_on_input = Some(reg);
    }

    fn set_delay_timer(&mut self, reg: u8) {
        self.regs.delay_timer = self.regs.get_data(reg);
    }

    fn set_sound_timer(&mut self, reg: u8) {
        self.regs.sound_timer = self.regs.get_data(reg);
    }

    fn add_to_index(&mut self, reg: u8) {
        self.regs.index = self.regs.index.wrapping_add(self.regs.get_data(reg) as u16);
    }

    fn set_index_to_character(&mut self, reg: u8) {
        self.regs.index = self.regs.get_data(reg) as u16 * 5
    }

    fn store_bcd(&mut self, reg: u8) {
        let val = self.regs.get_data(reg);

        let hundreds = val / 100;
        let tens = (val % 100) / 10;
        let ones = val % 10;

        self.mem.write(self.regs.index, hundreds);
        self.mem.write(self.regs.index.wrapping_add(1), tens);
        self.mem.write(self.regs.index.wrapping_add(2), ones);
    }

    fn store_to_index(&mut self, reg: u8) {
        for n in 0..reg {
            self.mem.write(self.regs.index.wrapping_add(n as u16), self.regs.get_data(n));
        }
    }

    fn fill_from_index(&mut self, reg: u8) {
        for n in 0..reg {
            let val = self.mem.read(self.regs.index.wrapping_add(n as u16));
            self.regs.set_data(n, val);
        }
    }

    fn read_opcode(&mut self) -> (u8, u8, u8, u8) {
        let word = self.mem.read_word(self.regs.address);

        self.regs.address = self.regs.address.wrapping_add(2);

        let nib1 = word & 0xF;
        let nib2 = (word & 0xF0) >> 4;
        let nib3 = (word & 0xF00) >> 8;
        let nib4 = (word & 0xF000) >> 12;

        (nib4 as u8, nib3 as u8, nib2 as u8, nib1 as u8)
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
        self.regs.stack = self.regs.stack.wrapping_add(1);
    }

    fn push_addr(&mut self, address: u16) {
       let x = (address & 0xFF) as u8;
       let y = ((address >> 8) & 0xFF) as u8;
       self.push(x);
       self.push(y);
    }

    fn pop(&mut self) -> u8 {
        self.regs.stack = self.regs.stack.wrapping_sub(1);
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

    fn toggle_pixel(&mut self, pixel: u8,  x: u8, y: u8) -> bool {
        let real_x = (x & 0x3F) as u16;
        let real_y = (y & 0x1F) as u16;
        let offset = ((real_y * 64) + real_x) as usize;
        let flipped = self.screen[offset] != 0 && pixel != 0;
        self.screen[offset] = pixel ^ self.screen[offset];
        flipped
    }

    fn draw_line(&mut self, line: u8, x: u8, y: u8) -> bool {
        let mut flipped = false;
        for n in 0..8 {
            flipped |= self.toggle_pixel(((line << n) & 0x80) >> 7, x + n, y);
        }
        flipped
    }
}

struct Registers {
    data: [u8;16],
    address: u16,
    stack: u16,
    index: u16,
    delay_timer: u8,
    sound_timer: u8,
}

impl Registers { 
    fn new() -> Registers {
        Registers {
            data: [0; 16],
            address: 0x200,
            stack: 0xEA0,
            index: 0,
            delay_timer: 0,
            sound_timer: 0,
        }
    }

    fn get_data(&self, ind: u8) -> u8 {
        self.data[(ind & 0xF) as usize]
    }

    fn set_data(&mut self, ind: u8, value: u8) {
        self.data[(ind & 0xF) as usize] = value;
    }
}

struct Memory {
    bytes: [u8;0x1000],
}

impl Memory {
    fn new() -> Memory {
        Memory {
            bytes: [0; 0x1000]
        }
    }
    
    fn new_with_rom(rom: Vec<u8>) -> Memory {
        let mut bytes = [0; 0x1000];
        for x in 0..rom.len() {
            bytes[x + 0x200] = rom[x];
        }

        let font_bytes = [
            0xF0, 0x90, 0x90, 0x90, 0xF0, // 0
            0x20, 0x60, 0x20, 0x20, 0x70, // 1
            0xF0, 0x10, 0xF0, 0x80, 0xF0, // 2
            0xF0, 0x10, 0xF0, 0x10, 0xF0, // 3
            0x90, 0x90, 0xF0, 0x10, 0x10, // 4
            0xF0, 0x80, 0xF0, 0x10, 0xF0, // 5
            0xF0, 0x80, 0xF0, 0x90, 0xF0, // 6
            0xF0, 0x10, 0x20, 0x40, 0x40, // 7
            0xF0, 0x90, 0xF0, 0x90, 0xF0, // 8
            0xF0, 0x90, 0xF0, 0x10, 0xF0, // 9
            0xF0, 0x90, 0xF0, 0x90, 0x90, // A
            0xE0, 0x90, 0xE0, 0x90, 0xE0, // B
            0xF0, 0x80, 0x80, 0x80, 0xF0, // C
            0xE0, 0x90, 0x90, 0x90, 0xE0, // D
            0xF0, 0x80, 0xF0, 0x80, 0xF0, // E
            0xF0, 0x80, 0xF0, 0x80, 0x80  // F
            
        ];

        for x in 0..font_bytes.len() {
            bytes[x] = font_bytes[x];
        }

        Memory {
            bytes: bytes,
        }
    }

    fn read(&self, addr: u16) -> u8 {
        let safe_addr = addr & 0xFFF;
        self.bytes[safe_addr as usize]
    }

    fn read_word(&self, addr: u16) -> u16 {
        (self.read(addr.wrapping_add(1)) as u16) | ((self.read(addr) as u16) << 8)
    }

    fn write(&mut self, addr: u16, value: u8) {
        let safe_addr = addr & 0xFFF;
        self.bytes[safe_addr as usize] = value;
    }
}

struct ConsoleRenderer;

impl ConsoleRenderer {
    fn new() -> ConsoleRenderer {
        ConsoleRenderer
    }
}

impl Chip8System for ConsoleRenderer {
    fn render(&mut self, screen: &[u8; 2048]) {
        let mut s = String::from("\x1b[2J\x1b[1;1H");
        for n in 0..2048 {
            if n % 64 == 0 && n != 0 {
                s.push_str("\n");
            }
            s.push_str(&format!("{0}", screen[n]));
        }

        let stdout = io::stdout();
        let mut handle = stdout.lock();

        let _ = handle.write_all(s.as_bytes());
        let _ = handle.flush();

        ::std::thread::sleep_ms(160);
    }

    fn get_input(&mut self) -> Option<u8> {
        None
    }

    fn is_closed(&mut self) -> bool {
        false
    }
}
