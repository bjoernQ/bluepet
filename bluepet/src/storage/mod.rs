use core::ops::Index;

use embedded_hal::blocking::spi::Transfer;
use spi_memory::prelude::*;
use spi_memory::series25::Flash;

use embedded_hal::digital::v2::OutputPin;

#[derive(Debug, Copy, Clone)]
struct Filename {
    filename: [u8; 16],
}

impl Filename {
    fn empty() -> Filename {
        Filename {
            filename: [0u8; 16],
        }
    }

    fn new(name: [u8; 16]) -> Filename {
        Filename { filename: name }
    }

    fn byte_at(&self, index: usize) -> u8 {
        self.filename[index]
    }
}

impl PartialEq for Filename {
    fn eq(&self, other: &Self) -> bool {
        for i in 0usize..16usize {
            if self.filename[i] != other.filename[i] {
                return false;
            }
        }

        true
    }
}

impl Index<usize> for Filename {
    type Output = u8;

    fn index(&self, index: usize) -> &u8 {
        &self.filename[index]
    }
}

#[derive(Debug, Copy, Clone)]
pub struct FileEntry {
    filename: Filename,
    address: u32,
    len: u16,
}

impl FileEntry {
    fn empty() -> FileEntry {
        FileEntry {
            filename: Filename::empty(),
            address: 0,
            len: 0,
        }
    }

    fn new(filename: Filename, address: u32, len: u16) -> FileEntry {
        FileEntry {
            filename: filename,
            address: address,
            len: len,
        }
    }

}

struct NotFound;

const BUFFER_SIZE: usize = 1024; // not enough RAM to use 4096 bytes of buffer
const SECTOR_SIZE: usize = 4096;
const MAX_FILENAME_LEN: usize = 16;
const MAX_FILES: usize = 46;

const MAX_FILE_SIZE: usize = 8192;
const DIRECTORY_BYTES: usize = 1024;

const DIR_ENTRY_BASIC_LINE_SIZE: usize = 4 + 2 + 1 + 16 + 1 + 1;

pub struct FlashStorage<SPI: Transfer<u8>, CS: OutputPin> {
    flash: Flash<SPI, CS>,

    filename: [u8; MAX_FILENAME_LEN],
    filename_index: usize,

    buffer: [u8; BUFFER_SIZE],
    buffer_valid: bool,
    buffer_has_save_data: bool,

    flash_address: u32,
    buffer_address: u32,

    pub directory: [FileEntry; MAX_FILES],
    directory_loaded: bool,

    current_fileno: usize,

    file_byte_count: usize,

    directory_valid_entries: usize,

    loading_directory: bool,
    loading_directory_fileno: usize,
    loading_directory_filename_index: usize, 
    loading_directory_fileindex: usize,
}


impl<SPI: Transfer<u8>, CS: OutputPin> FlashStorage<SPI, CS> {
    pub fn new(spi_flash: Flash<SPI, CS>) -> FlashStorage<SPI, CS> {
        FlashStorage {
            flash: spi_flash,

            filename: [0u8; MAX_FILENAME_LEN],
            filename_index: 0usize,

            buffer: [0u8; BUFFER_SIZE],
            buffer_valid: false,
            buffer_has_save_data: false,

            flash_address: 0u32,
            buffer_address: 0u32,

            directory: [FileEntry::empty(); MAX_FILES],
            directory_loaded: false,

            current_fileno: 0usize,

            file_byte_count: 0usize,

            directory_valid_entries: 0,

            loading_directory: false,
            loading_directory_fileno: 0,
            loading_directory_filename_index: 0,
            loading_directory_fileindex: 0,
        }
    }

    fn flush(&mut self) {
        self.flash.erase_sectors(self.buffer_address, 1).unwrap_or_default();
        self.flash.write_bytes(self.buffer_address, &mut self.buffer).unwrap_or_default();
        self.flash.read(self.buffer_address, &mut self.buffer).unwrap_or_default();
        self.buffer_has_save_data = false;
    }

    fn put_byte(&mut self, address: u32, value: u8) {
        let flash_address = address % BUFFER_SIZE as u32 + (address / BUFFER_SIZE as u32) * SECTOR_SIZE as u32;
        if flash_address < self.buffer_address
            || flash_address > self.buffer_address + self.buffer.len() as u32
            || !self.buffer_valid || !self.buffer_has_save_data
        {
            if self.buffer_has_save_data {
                self.flush();
            }

            for i in 0..self.buffer.len() {
                self.buffer[i] = 0u8;
            }

            self.buffer_address = (address / BUFFER_SIZE as u32) * SECTOR_SIZE as u32;
            self.flash
                .read(self.buffer_address, &mut self.buffer)
                .unwrap_or_default();
            self.buffer_valid = true;
            self.buffer_has_save_data = true;
        }

        self.buffer[(flash_address - self.buffer_address) as usize] = value;
    }

    fn put_word(&mut self, address: u32, value: u32) {
       self.put_byte(address, (value >> 24) as u8);
       self.put_byte(address+1, (value >> 16) as u8);
       self.put_byte(address+2, (value >> 8) as u8);
       self.put_byte(address+3, value as u8);
    }

    fn put_hword(&mut self, address: u32, value: u16) {
        self.put_byte(address, (value >> 8) as u8);
        self.put_byte(address+1, value as u8);
     }
 
    fn get_byte(&mut self, address: u32) -> u8 {
        let flash_address = address % BUFFER_SIZE as u32 + (address / BUFFER_SIZE as u32) * SECTOR_SIZE as u32;
        if flash_address < self.buffer_address
            || flash_address > self.buffer_address + self.buffer.len() as u32
            || !self.buffer_valid
        {
            if self.buffer_has_save_data {
                self.flush();
            }

            self.buffer_address = (address / BUFFER_SIZE as u32) * SECTOR_SIZE as u32;
            self.flash
                .read(self.buffer_address, &mut self.buffer)
                .unwrap_or_default();
            self.buffer_valid = true;
            self.buffer_has_save_data = false;
        }

        self.buffer[(flash_address - self.buffer_address) as usize]
    }

    fn get_word(&mut self, address: u32) -> u32 {
        ((self.get_byte(address) as u32) << 24)
            + ((self.get_byte(address + 1) as u32) << 16)
            + ((self.get_byte(address + 2) as u32) << 8)
            + (self.get_byte(address + 3) as u32)
    }

    fn get_hword(&mut self, address: u32) -> u16 {
        ((self.get_byte(address) as u16) << 8) + (self.get_byte(address + 1) as u16)
    }

    fn find_file(&mut self, file_name: [u8; 16]) -> Result<(FileEntry, usize), NotFound> {
        self.ensure_directory();

        let file_to_find = Filename::new(file_name);
        for i in 0..46 {
            if self.directory[i].filename == file_to_find {
                return Ok( (self.directory[i],i) );
            }
        }

        Err(NotFound {})
    }

    pub fn ensure_directory(&mut self) {
        if !self.directory_loaded {
            let mut tmp_directory = [FileEntry::empty(); MAX_FILES];
            for i in 0..MAX_FILES {
                let mut filename = [0u8; 16];
                for j in 0usize..16usize {
                    filename[j] =
                        self.get_byte((0 as u32 + i as u32 * 22 as u32 + j as u32) as u32);
                }
                let file_address = self.get_word((0 + i * 22 + 16) as u32);
                let file_len = self.get_hword((0 + i * 22 + 20) as u32);
                tmp_directory[i] = FileEntry::new(Filename::new(filename), file_address, file_len);
            }
            self.directory = tmp_directory;
            self.directory_loaded = true;
        }
    }

    fn put_directory_entry(&mut self, index: usize, entry: FileEntry) {
        self.directory[index] = entry;

        let i = index;
        for j in 0..16 {
            self.put_byte((0 as u32 + i as u32 * 22 as u32 + j as u32) as u32, entry.filename.byte_at(j));
        }
        self.put_word((0 + i * 22 + 16) as u32, entry.address);
        self.put_hword((0 + i * 22 + 20) as u32, entry.len);
    }

    fn add_file(&mut self, file_name: [u8; 16]) -> usize {
        self.ensure_directory();
        let fname = Filename::new(file_name);

        for i in 0..46 {
            let entry = self.directory[i];

            if entry.address == 0 || entry.address == 0xffffffff || entry.filename == fname {
                let address = i * MAX_FILE_SIZE + DIRECTORY_BYTES;
                self.put_directory_entry(i, FileEntry::new(Filename::new(file_name), address as u32, 0));
                return i;
            }
        }

        9999
    }

    fn set_filesize(&mut self, fileno: usize, filesize: u16) {
        self.put_hword((fileno * 22 + 20) as u32, filesize);
        self.flush();
        self.directory_loaded = false;
    }
}


impl<SPI: Transfer<u8>, CS: OutputPin> pet::io::Storage for FlashStorage<SPI, CS> {
    fn start_filename(&mut self) {
        self.filename[0] = 0;
        self.filename[1] = 0;
        self.filename[2] = 0;
        self.filename[3] = 0;
        self.filename[4] = 0;
        self.filename[5] = 0;
        self.filename[6] = 0;
        self.filename[7] = 0;
        self.filename[8] = 0;
        self.filename[9] = 0;
        self.filename[10] = 0;
        self.filename[11] = 0;
        self.filename[12] = 0;
        self.filename[13] = 0;
        self.filename[14] = 0;
        self.filename[15] = 0;
        self.filename_index = 0;
        self.file_byte_count = 0;
    }

    fn next_filename_byte(&mut self, value: u8) {
        self.filename[self.filename_index] = value;
        self.filename_index += 1;
    }

    fn fname_done(&mut self) {
        if self.filename[0] == '$' as u8 {
            self.loading_directory = true;

            for i in 0..MAX_FILES {
                if self.directory[i].address != 0xffff && self.directory[i].address!=0x0 {
                    self.loading_directory_fileno = i;
                    break;
                } 
            }

            self.loading_directory_filename_index = 0;

            return;
        }

        self.loading_directory = false;
        match self.find_file(self.filename){
            Ok((_, fileno)) => { self.current_fileno = fileno; }
            Err(_) => { self.current_fileno = 9999; }
        }
    }


    fn start_save(&mut self) {
        let fileno = self.add_file(self.filename);

        if fileno != 9999 {
            self.current_fileno = fileno;
        }
    }

    fn end_save(&mut self) {
        self.flush();
        self.set_filesize(self.current_fileno, self.file_byte_count as u16);
    }

    fn has_data_to_load(&mut self) -> bool {
        if self.loading_directory {
            return true;
        }

        self.load_data_len() > 0
    }

    fn load_data_byte(&mut self, index: usize) -> u8 {
        self.file_byte_count = index + 1;

        if self.loading_directory {
            let current_entry = if self.loading_directory_fileno != 9999 {
                Some(&self.directory[self.loading_directory_fileno])
            } else {
                None
            };

            fn calculate_next_basic_line_ptr(loading_directory_fileindex: usize, entry_count: usize) -> usize {
                if loading_directory_fileindex >= entry_count {
                    0
                } else {
                    0x401usize + (loading_directory_fileindex + 1) * DIR_ENTRY_BASIC_LINE_SIZE
                }
            }

            fn filename_byte_in_basic(entry: Option<&FileEntry>, index: usize) -> u8 {
                match entry {
                    Some(entry) => {
                        let filename = entry.filename;
                        
                        if index >= 16 {
                            if index > 0 && filename[15] != 0 {
                                return '"' as u8;
                            } else {
                                return ' ' as u8;
                            }
                        }

                        let b = filename[index];

                        if b != 0 {
                            b
                        } else {
                            if index > 0 && filename[index - 1] != 0 {
                                '"' as u8
                            } else {
                                ' ' as u8
                            }
                        }
        
                    }
                    None => { 0 }
                }
                
            }

            fn len_or_0(v: Option<&FileEntry>) -> u16 {
                match v {
                    Some(entry) => entry.len,
                    _ => 0
                }
            }

            if index == 0 {
                return 0x01;
            }

            if index == 1 {
                return 0x04;
            }

            let res = match self.loading_directory_filename_index {
                0 => (((calculate_next_basic_line_ptr(self.loading_directory_fileindex, self.directory_valid_entries))) & 0xff) as u8,
                1 => ((((calculate_next_basic_line_ptr(self.loading_directory_fileindex, self.directory_valid_entries))) & 0xff00) >> 8) as u8,

                2 => ((len_or_0(current_entry)) & 0xff) as u8,
                3 => (((len_or_0(current_entry)) & 0xff00) >> 8) as u8,

                4..=5 => ' ' as u8,

                6 => '"' as u8,

                7..=23 => filename_byte_in_basic(current_entry, self.loading_directory_filename_index - 7),
            
                24 => 0u8,

                _ => 0u8,
            };

            self.loading_directory_filename_index += 1;

            if self.loading_directory_filename_index > DIR_ENTRY_BASIC_LINE_SIZE - 1 {
                self.loading_directory_filename_index = 0;
                self.loading_directory_fileindex += 1;

                let start_at = self.loading_directory_fileno + 1;
                self.loading_directory_fileno = 9999;
                for i in start_at..MAX_FILES {
                    if self.directory[i].address != 0xffffffff && self.directory[i].address!=0x0 {
                        self.loading_directory_fileno = i;
                    } 
                }
            }

            return res;
        }



        let res = self.get_byte(self.current_fileno as u32 * MAX_FILE_SIZE as u32 + DIRECTORY_BYTES as u32 + index as u32);
        res
    }

    fn save_data_byte(&mut self, index: usize, value: u8) {
        self.file_byte_count = index + 1;
        self.put_byte(self.current_fileno as u32 * MAX_FILE_SIZE as u32 + DIRECTORY_BYTES as u32 + index as u32, value);
    }

    fn load_data_len(&mut self) -> usize {
        if self.loading_directory {
            self.ensure_directory();
            let mut entries = 0;
            for entry in self.directory.iter() {
                if entry.address != 0xffffffff && entry.address != 0x0 {
                    entries += 1;
                }
            }
            self.directory_valid_entries = entries;
            return entries * DIR_ENTRY_BASIC_LINE_SIZE + 2 + 2;
        }

        let entry = self.find_file(self.filename);
        match entry {
            Ok(entry) => entry.0.len as usize,
            Err(_) => 0usize,
        }
    }
}
