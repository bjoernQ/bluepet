#![no_std]

use mos6502::Memory;

use core::cell::RefCell;

pub mod io;
use io::Io;
use io::Keyboard;
use io::Storage;

const ROM_C000: &'static [u8] = include_bytes!("../rom/rom-b-c000.bin");
const ROM_D000: &'static [u8] = include_bytes!("../rom/rom-b-d000.bin");
const ROM_E000: &'static [u8] = include_bytes!("../rom/rom-e-e000.bin");
const ROM_F000: &'static [u8] = include_bytes!("../rom/rom-k-f000.bin");

pub struct Ram<'a> {
    pub ram: RefCell<&'a mut [u8; 8192]>,
    pub vid_ram: RefCell<&'a mut [u8; 2048]>,
    pub io: RefCell<Io<'a>>,
}

impl<'a> Ram<'a> {
    pub fn new(
        ram: &'a mut [u8; 8192],
        vid_ram: &'a mut [u8; 2048],
        storage: &'a mut dyn Storage,
    ) -> Ram<'a> {
        let mut io = Io::new(Keyboard::new(), storage);
        io.reset();

        Ram {
            ram: RefCell::new(ram),
            vid_ram: RefCell::new(vid_ram),
            io: RefCell::new(io),
        }
    }
}

impl<'a> Memory for Ram<'a> {
    fn get(&self, addr: u16) -> u8 {
        if addr >= 0xe800 && addr < 0xe850 {
            return self.io.borrow_mut().read(addr - 0xe800);
        }

        if addr >= 0x8000 && addr < 0x8800 {
            return self.vid_ram.borrow()[(addr - 0x8000) as usize];
        }
        if addr >= 0xc000 && addr < 0xd000 {
            return ROM_C000[(addr - 0xc000) as usize];
        }
        if addr >= 0xd000 && addr < 0xe000 {
            return ROM_D000[(addr - 0xd000) as usize];
        }
        if addr >= 0xe000 && addr < 0xf000 {
            return ROM_E000[(addr - 0xe000) as usize];
        }
        if addr >= 0xf000 {
            return ROM_F000[(addr - 0xf000) as usize];
        }

        if addr >= 0x2000 {
            return 0;
        }

        self.ram.borrow()[addr as usize]
    }

    fn set(&self, addr: u16, v: u8) {
        if addr >= 0xe800 && addr < 0xe850 {
            self.io.borrow_mut().write(addr - 0xe800, v);
            return;
        }

        if addr >= 0x8000 && addr < 0x8800 {
            self.vid_ram.borrow_mut()[(addr - 0x8000) as usize] = v;
            return;
        }

        if addr >= 0x2000 {
            return;
        }

        self.ram.borrow_mut()[addr as usize] = v;
    }
}

#[cfg(test)]
mod tests {
    extern crate std;
    use std::println;
    use std::string::String;
    use mos6502::Cpu;

    use super::*;

    const CHARCONV: [u16; 306] = [
        0x00, 0x0040, 0x01, 0x0041, 0x02, 0x0042, 0x03, 0x0043, 0x04, 0x0044, 0x05, 0x0045, 0x06,
        0x0046, 0x07, 0x0047, 0x08, 0x0048, 0x09, 0x0049, 0x0A, 0x004A, 0x0B, 0x004B, 0x0C, 0x004C,
        0x0D, 0x004D, 0x0E, 0x004E, 0x0F, 0x004F, 0x10, 0x0050, 0x11, 0x0051, 0x12, 0x0052, 0x13,
        0x0053, 0x14, 0x0054, 0x15, 0x0055, 0x16, 0x0056, 0x17, 0x0057, 0x18, 0x0058, 0x19, 0x0059,
        0x1A, 0x005A, 0x1B, 0x005B, 0x1C, 0x005C, 0x1D, 0x005D, 0x1E, 0x2191, 0x1F, 0x2190, 0x20,
        0x0020, 0x21, 0x0021, 0x22, 0x0022, 0x23, 0x0023, 0x24, 0x0024, 0x25, 0x0025, 0x26, 0x0026,
        0x27, 0x0027, 0x28, 0x0028, 0x29, 0x0029, 0x2A, 0x002A, 0x2B, 0x002B, 0x2C, 0x002C, 0x2D,
        0x002D, 0x2E, 0x002E, 0x2F, 0x002F, 0x30, 0x0030, 0x31, 0x0031, 0x32, 0x0032, 0x33, 0x0033,
        0x34, 0x0034, 0x35, 0x0035, 0x36, 0x0036, 0x37, 0x0037, 0x38, 0x0038, 0x39, 0x0039, 0x3A,
        0x003A, 0x3B, 0x003B, 0x3C, 0x003C, 0x3D, 0x003D, 0x3E, 0x003E, 0x3F, 0x003F, 0x40, 0x2500,
        0x41, 0x0061, 0x42, 0x0062, 0x43, 0x0063, 0x44, 0x0064, 0x45, 0x0065, 0x46, 0x0066, 0x47,
        0x0067, 0x48, 0x0068, 0x49, 0x0069, 0x4A, 0x006A, 0x4B, 0x006B, 0x4C, 0x006C, 0x4D, 0x006D,
        0x4E, 0x006E, 0x4F, 0x006F, 0x50, 0x0071, 0x51, 0x0072, 0x52, 0x0072, 0x53, 0x0073, 0x54,
        0x0074, 0x55, 0x0075, 0x56, 0x0076, 0x57, 0x0077, 0x58, 0x0078, 0x59, 0x0079, 0x5A, 0x007A,
        0x5B, 0x253C, 0x5C, 0x258C, 0x5D, 0x2502, 0x5E, 0x2591, 0x5F, 0x25A7, 0x60, 0x0020, 0x61,
        0x258C, 0x62, 0x2584, 0x63, 0x2594, 0x64, 0x2581, 0x65, 0x258F, 0x66, 0x2592, 0x67, 0x2595,
        0x68, 0x2584, 0x69, 0x25A8, 0x6A, 0x2595, 0x6B, 0x251C, 0x6C, 0x2597, 0x6D, 0x2514, 0x6E,
        0x2510, 0x6F, 0x2582, 0x70, 0x250C, 0x71, 0x2534, 0x72, 0x252C, 0x73, 0x2524, 0x74, 0x258E,
        0x75, 0x258D, 0x76, 0x2590, 0x77, 0x2594, 0x78, 0x2580, 0x79, 0x2583, 0x7A, 0x2713, 0x7B,
        0x2596, 0x7C, 0x259D, 0x7D, 0x2518, 0x7E, 0x2598, 0x7F, 0x259A, 0xDE, 0x2591, 0xDF, 0x25A7,
        0xE0, 0x2588, 0xE1, 0x2590, 0xE2, 0x2580, 0xE3, 0x2587, 0xE4, 0x2580, 0xE5, 0x2598, 0xE6,
        0x2592, 0xE7, 0x2589, 0xE9, 0x25A8, 0xEA, 0x258A, 0xEC, 0x259B, 0xEF, 0x2580, 0xF4, 0x2590,
        0xF5, 0x2590, 0xF6, 0x258B, 0xF7, 0x2586, 0xF8, 0x2585, 0xF9, 0x2580, 0xFB, 0x259C, 0xFC,
        0x2599, 0xFE, 0x259F, 0xFF, 0x259E, 0xA0, 0x2588,
    ];

    // see http://www.6502.org/users/andre/petindex/keyboards.html#graph
    const ASCII2PET: [u8; 8 * 10 * 2] = [
        '!' as u8, 0x00, '#' as u8, 0x01, '%' as u8, 0x02, '&' as u8, 0x03, '(' as u8, 0x04,
        127 as u8, 0x05, // BACKSPACE?
        0 as u8, 0x06, // HOME
        0 as u8, 0x07, // CRSR RIGHT
        '"' as u8, 0x10, '$' as u8, 0x11, '\'' as u8, 0x12, '\\' as u8, 0x13, ')' as u8, 0x14,
        '~' as u8, 0x15, // UNASSIGNED
        0 as u8, 0x16, // CRSR DOWN
        127 as u8, 0x17, // DELETE?
        'q' as u8, 0x20, 'e' as u8, 0x21, 't' as u8, 0x22, 'u' as u8, 0x23, 'o' as u8, 0x24,
        '^' as u8, 0x25, '7' as u8, 0x26, '9' as u8, 0x27, 'w' as u8, 0x30, 'r' as u8, 0x31,
        'y' as u8, 0x32, 'i' as u8, 0x33, 'p' as u8, 0x34, '~' as u8, 0x35, '8' as u8, 0x36,
        '/' as u8, 0x37, 'a' as u8, 0x40, 'd' as u8, 0x41, 'g' as u8, 0x42, 'j' as u8, 0x43,
        'l' as u8, 0x44, '~' as u8, 0x45, '4' as u8, 0x46, '6' as u8, 0x47, 's' as u8, 0x50,
        'f' as u8, 0x51, 'h' as u8, 0x52, 'k' as u8, 0x53, ':' as u8, 0x54, '~' as u8, 0x55,
        '5' as u8, 0x56, '*' as u8, 0x57, 'z' as u8, 0x60, 'c' as u8, 0x61, 'b' as u8, 0x62,
        'm' as u8, 0x63, ';' as u8, 0x64, '\r' as u8, 0x65, '1' as u8, 0x66, '3' as u8, 0x67,
        'x' as u8, 0x70, 'v' as u8, 0x71, 'n' as u8, 0x72, ',' as u8, 0x73, '?' as u8, 0x74,
        '~' as u8, 0x75, '2' as u8, 0x76, '+' as u8, 0x77, '~' as u8, 0x80, // LEFT SHIFT
        '@' as u8, 0x81, ']' as u8, 0x82, '~' as u8, 0x83, '>' as u8, 0x84, '~' as u8,
        0x85, // RIGHT SHIFT
        '0' as u8, 0x86, '-' as u8, 0x87, '~' as u8, 0x90, // REVERSE ON?
        '[' as u8, 0x91, ' ' as u8, 0x92, '<' as u8, 0x93, '~' as u8, 0x94, // STOP
        '~' as u8, 0x95, '.' as u8, 0x96, '=' as u8, 0x97,
    ];

    fn screen_as_string(mem: &Ram) -> String {
        let mut res = String::new();
        let mut x = 0;
        for addr in 0x8000usize..0x8400usize {
            let v = mem.get(addr as u16);

            let mut ascii: char = 32 as char;
            for i in (0..306).step_by(2) {
                if v == CHARCONV[i] as u8 {
                    ascii = CHARCONV[i + 1] as u8 as char;
                }
            }

            res.push(ascii);

            x += 1;
            if x == 40 {
                x = 0;
                res.push('\n');
            }
        }

        res
    }

    struct TestStorage {
        filename: [u8; 16],
        load_data_length: usize,
        load_data: [u8; 13],
        save_data: [u8; 256],

        filename_index: usize,
    }

    impl TestStorage {
        fn new() -> TestStorage {
            TestStorage {
                filename: [0u8; 16],
                load_data_length: 13usize,
                load_data: [
                    0x01, 0x04, 0x0a, 0x04, 0x64, 0x00, 0x8f, 0x20, 0x48, 0x49, 0x00, 0x00, 0x00,
                ],
                save_data: [0u8; 256],

                filename_index: 0usize,
            }
        }
    }

    impl Storage for TestStorage {
        fn start_filename(&mut self) {
            println!("start filename");
            self.filename_index = 0;
        }

        fn next_filename_byte(&mut self, value: u8) {
            println!("next filename byte {} = char {}", value, value as char);
            self.filename[self.filename_index] = value;
        }

        fn start_save(&mut self) {
            println!("save start");
        }

        fn has_data_to_load(&mut self) -> bool {
            println!("has_data_to_load");
            self.load_data_length > 0
        }

        fn load_data_byte(&mut self, index: usize) -> u8 {
            println!("load data {}", index);
            self.load_data[index]
        }

        fn save_data_byte(&mut self, index: usize, value: u8) {
            println!("save byte {} {}", index, value);
            self.save_data[index] = value;
        }

        fn load_data_len(&mut self) -> usize {
            println!("load data len");
            self.load_data.len()
        }

        fn end_save(&mut self) {
            println!("end save");
        }

        fn fname_done(&mut self) {
            println!("filename done");
        }
    }

    #[test]
    fn it_works() {
        let mut test_storage = TestStorage::new();
        let enter_str = "10 for i=0 to 15\r20 print \"hello world\" i\r30 next\rrun\r";
        let mut str_idx = 0;
        let mut keyboard_cntr = 50000;

        let mut ram = [0u8; 8192];
        let mut vid_ram = [0u8; 2048];
        let mem = Ram::new(&mut ram, &mut vid_ram, &mut test_storage);
        let mut cpu = Cpu::new(&mem);
        cpu.reset();

        let mut cycle_cnt: u64 = 0;

        let mut tick_cntr = 0u32;
        let mut cnt = 0;
        for _ in 0..6393000 {
            if cycle_cnt == 0 {
                cnt += 1;
                cycle_cnt = cpu.step();
                cycle_cnt -= 1;
            } else {
                cycle_cnt -= 1;
            }

            tick_cntr += 1;
            if tick_cntr >= 1000 {
                tick_cntr = 0;
                let trigger_irq = mem.io.borrow_mut().tick();
                if trigger_irq {
                    cpu.trigger_irq();
                }
            }

            // simulate entering a string
            if keyboard_cntr == 30000 {
                if str_idx < enter_str.len() {
                    let mut row_col = 0xff;
                    let c = *&enter_str.as_bytes()[str_idx];
                    for i in (0..ASCII2PET.len()).step_by(2) {
                        if c == ASCII2PET[i] {
                            row_col = ASCII2PET[i + 1];
                        }
                    }
                    if row_col != 0xff {
                        mem.io.borrow_mut().keyboard.key_down(row_col);
                    }
                }
            }
            if keyboard_cntr == 0 {
                if str_idx < enter_str.len() {
                    let mut row_col = 0xff;
                    let c = *&enter_str.as_bytes()[str_idx];
                    for i in (0..ASCII2PET.len()).step_by(2) {
                        if c == ASCII2PET[i] {
                            row_col = ASCII2PET[i + 1];
                        }
                    }
                    if row_col != 0xff {
                        mem.io.borrow_mut().keyboard.key_up(row_col);
                    }
                    str_idx += 1;
                }
                keyboard_cntr = 50000;
            }
            keyboard_cntr -= 1;
        }

        let screen_content = screen_as_string(&mem);
        println!("{}\n", screen_content);

        println!("{}", cnt);

        assert!(screen_content.contains("HELLO WORLD 10"));
    }

    #[test]
    fn load_from_disk_works() {
        let mut test_storage = TestStorage::new();

        let second_str = "list\r";

        let mut enter_str = "load \"test\",8\r";
        let mut str_idx = 0;

        let mut keyboard_cntr = 50000;

        let mut ram = [0u8; 8192];
        let mut vid_ram = [0u8; 2048];
        let mem = Ram::new(&mut ram, &mut vid_ram, &mut test_storage);
        let mut cpu = Cpu::new(&mem);
        cpu.reset();

        println!("{:x} {:x} {:x} {:x} {:x} {:x} {:x} {:x} {:x} {:x} {:x} {:x} {:x} {:x} {:x} {:x} {:x} {:x} {:x} {:x} {:x} {:x} {:x} {:x}", 
        mem.get(0x400), mem.get(0x401), mem.get(0x402), mem.get(0x403),
        mem.get(0x404), mem.get(0x405), mem.get(0x406), mem.get(0x407),
        mem.get(0x408), mem.get(0x409), mem.get(0x40a), mem.get(0x40b),

        mem.get(0x40c), mem.get(0x40d), mem.get(0x40e), mem.get(0x40f),
        mem.get(0x410), mem.get(0x411), mem.get(0x412), mem.get(0x413),
        mem.get(0x414), mem.get(0x415), mem.get(0x416), mem.get(0x417),

    );

        let mut cycle_cnt: u64 = 0;

        let mut tick_cntr = 0u32;
        let mut cnt = 0;
        for i in 0..9393000 {
            if i == 5393000 {
                enter_str = second_str;
                str_idx = 0;
                keyboard_cntr = 50000;
            }

            if cycle_cnt == 0 {
                cnt += 1;
                cycle_cnt = cpu.step();
                cycle_cnt -= 1;
            } else {
                cycle_cnt -= 1;
            }

            tick_cntr += 1;
            if tick_cntr >= 1000 {
                tick_cntr = 0;
                let trigger_irq = mem.io.borrow_mut().tick();
                if trigger_irq {
                    cpu.trigger_irq();
                }
            }

            // simulate entering a string
            if keyboard_cntr == 30000 {
                if str_idx < enter_str.len() {
                    let mut row_col = 0xff;
                    let c = *&enter_str.as_bytes()[str_idx];
                    for i in (0..ASCII2PET.len()).step_by(2) {
                        if c == ASCII2PET[i] {
                            row_col = ASCII2PET[i + 1];
                        }
                    }
                    if row_col != 0xff {
                        mem.io.borrow_mut().keyboard.key_down(row_col);
                    }
                }
            }
            if keyboard_cntr == 0 {
                if str_idx < enter_str.len() {
                    let mut row_col = 0xff;
                    let c = *&enter_str.as_bytes()[str_idx];
                    for i in (0..ASCII2PET.len()).step_by(2) {
                        if c == ASCII2PET[i] {
                            row_col = ASCII2PET[i + 1];
                        }
                    }
                    if row_col != 0xff {
                        mem.io.borrow_mut().keyboard.key_up(row_col);
                    }
                    str_idx += 1;
                }
                keyboard_cntr = 50000;
            }
            keyboard_cntr -= 1;
        }

        let screen_content = screen_as_string(&mem);
        println!("{}\n", screen_content);

        println!("{}", cnt);

        println!("{:x} {:x} {:x} {:x} {:x} {:x} {:x} {:x} {:x} {:x} {:x} {:x} {:x} {:x} {:x} {:x} {:x} {:x} {:x} {:x} {:x} {:x} {:x} {:x}", 
        mem.get(0x400), mem.get(0x401), mem.get(0x402), mem.get(0x403),
        mem.get(0x404), mem.get(0x405), mem.get(0x406), mem.get(0x407),
        mem.get(0x408), mem.get(0x409), mem.get(0x40a), mem.get(0x40b),

        mem.get(0x40c), mem.get(0x40d), mem.get(0x40e), mem.get(0x40f),
        mem.get(0x410), mem.get(0x411), mem.get(0x412), mem.get(0x413),
        mem.get(0x414), mem.get(0x415), mem.get(0x416), mem.get(0x417),
        );

        assert!(screen_content.contains("100 REM HI"));
    }

    #[test]
    fn save_works() {
        let mut test_storage = TestStorage::new();

        let enter_str = "10 rem hello world\rsave\"test\",8\r";
        let mut str_idx = 0;
        let mut keyboard_cntr = 50000;

        let mut ram = [0u8; 8192];
        let mut vid_ram = [0u8; 2048];
        let mem = Ram::new(&mut ram, &mut vid_ram, &mut test_storage);
        let mut cpu = Cpu::new(&mem);
        cpu.reset();

        let mut cycle_cnt: u64 = 0;

        let mut tick_cntr = 0u32;
        let mut cnt = 0;
        for _ in 0..6393000 {
            if cycle_cnt == 0 {
                cnt += 1;
                cycle_cnt = cpu.step();
                cycle_cnt -= 1;
            } else {
                cycle_cnt -= 1;
            }

            tick_cntr += 1;
            if tick_cntr >= 1000 {
                tick_cntr = 0;
                let trigger_irq = mem.io.borrow_mut().tick();
                if trigger_irq {
                    cpu.trigger_irq();
                }
            }

            // simulate entering a string
            if keyboard_cntr == 30000 {
                if str_idx < enter_str.len() {
                    let mut row_col = 0xff;
                    let c = *&enter_str.as_bytes()[str_idx];
                    for i in (0..ASCII2PET.len()).step_by(2) {
                        if c == ASCII2PET[i] {
                            row_col = ASCII2PET[i + 1];
                        }
                    }
                    if row_col != 0xff {
                        mem.io.borrow_mut().keyboard.key_down(row_col);
                    }
                }
            }
            if keyboard_cntr == 0 {
                if str_idx < enter_str.len() {
                    let mut row_col = 0xff;
                    let c = *&enter_str.as_bytes()[str_idx];
                    for i in (0..ASCII2PET.len()).step_by(2) {
                        if c == ASCII2PET[i] {
                            row_col = ASCII2PET[i + 1];
                        }
                    }
                    if row_col != 0xff {
                        mem.io.borrow_mut().keyboard.key_up(row_col);
                    }
                    str_idx += 1;
                }
                keyboard_cntr = 50000;
            }
            keyboard_cntr -= 1;
        }

        let screen_content = screen_as_string(&mem);
        println!("{}\n", screen_content);

        println!("{}", cnt);

        assert!(test_storage.save_data[ 0]==0x01);
        assert!(test_storage.save_data[ 1]==0x04);
        assert!(test_storage.save_data[ 2]==0x13);
        assert!(test_storage.save_data[ 3]==0x04);
        assert!(test_storage.save_data[ 4]==0x0a);
        assert!(test_storage.save_data[ 5]==0x00);
        assert!(test_storage.save_data[ 6]==0x8f);
        assert!(test_storage.save_data[ 7]==0x20);
        assert!(test_storage.save_data[ 8]==0x48);
        assert!(test_storage.save_data[ 9]==0x45);
        assert!(test_storage.save_data[10]==0x4c);
        assert!(test_storage.save_data[11]==0x4c);
        assert!(test_storage.save_data[12]==0x4f);
        assert!(test_storage.save_data[13]==0x20);
        assert!(test_storage.save_data[14]==0x57);
        assert!(test_storage.save_data[15]==0x4f);
        assert!(test_storage.save_data[16]==0x52);
        assert!(test_storage.save_data[17]==0x4c);
        assert!(test_storage.save_data[18]==0x44);
    }
}
