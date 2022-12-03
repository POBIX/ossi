use crate::{io, pic, println};

// these ports assume we want to use the master ATA.
/// data register
const PORT_DR: u16 = 0x1F0;
/// error register
const PORT_ER: u16 = 0x1F1;
/// features register
const PORT_FR: u16 = 0x1F1;
/// sector count register
const PORT_SCR: u16 = 0x1F2;
/// sector number register / LBA low register
const PORT_SNR: u16 = 0x1F3;
/// cylinder low register / LBA mid register
const PORT_CLR: u16 = 0x1F4;
/// cylinder high register / LBA high register
const PORT_CHR: u16 = 0x1F5;
/// drive / head register
const PORT_DHR: u16 = 0x1F6;
/// status register
const PORT_SR: u16 = 0x1F7;
/// command register
const PORT_CR: u16 = 0x1F7;

// flags for the status register
/// busy executing a command
const STATUS_BSY: u8 = 0x80;
/// ready to accept a command
const STATUS_RDY: u8 = 0x40;
/// expecting/sending data
const STATUS_DRQ: u8 = 0x08;
/// error occurred
const STATUS_ERR: u8 = 0x01;

pub fn init() {
    use crate::interrupts::{self, GateType};
    unsafe {
        interrupts::IDT[pic::IRQ_OFFSET + 14] = interrupts::Handler::new(irq14, GateType::DInterrupt);
        interrupts::IDT[pic::IRQ_OFFSET + 15] = interrupts::Handler::new(irq15, GateType::DInterrupt);
    }
}

extern "x86-interrupt" fn irq14() {
    println!("14");
    pic::send_eoi(14);
}

extern "x86-interrupt" fn irq15() {
    println!("15");
    pic::send_eoi(15);
}

/// waits until flag is value in the status register.
#[inline] fn wait_for(flag: u8, value: bool) {
    unsafe {
        while (io::inb(PORT_SR) & flag == 0) == value {}
    }
}

fn setup_flags(lba: u32, sector_count: u8) {
    unsafe {
        // explanation for DHR value:
        // bits 0-3: bits 24-27 of the LBA block
        // bit 4: drive number
        // bit 5: always 1
        // bit 6: 1=use lba
        // bit 7: always 1
        io::outb(PORT_DHR, 0xE0 | (((lba >> 24) as u8) & 0xF));
        io::outb(PORT_SCR, sector_count);
        io::outb(PORT_SNR, lba as u8);
        io::outb(PORT_CLR, (lba >> 8) as u8);
        io::outb(PORT_CHR, (lba >> 16) as u8);
    }
}

const fn get_sector_count(arr: &[u8]) -> usize {
    if arr.len() % 512 != 0 {
        panic!("size of buffer in bytes must be divisible by 512");
    }
    arr.len() / 512
}

fn panic_if_error() {
    let error = unsafe { io::inb(PORT_ER) };
    if error != 0 {
        panic!("ATA read failed with error code {:0X}", error);
    }
}

pub unsafe fn read_sectors(lba: u32, buffer: &mut [u8]) {
    wait_for(STATUS_BSY, false);

    let sector_count = get_sector_count(buffer);

    setup_flags(lba, sector_count as u8);
    io::outb(PORT_CR, 0x20); // send the read command
    panic_if_error();

    // the disk sends out 16 bytes at a time.
    let buffer_u16 = core::mem::transmute::<&mut [u8], &mut [u16]>(buffer);

    for i in 0..sector_count {
        wait_for(STATUS_BSY, false);
        wait_for(STATUS_DRQ, true);
        for j in 0..256 {
            buffer_u16[256*i + j] = io::inw(PORT_DR);
        }
    }
}

pub unsafe fn write_sectors(lba: u32, data: &[u8]) {
    wait_for(STATUS_BSY, false);

    let sector_count = get_sector_count(data);

    setup_flags(lba, sector_count as u8);
    io::outb(PORT_CR, 0x30); // send the write command
    panic_if_error();

    // the disk receives 32 bytes at a time.
    let data_u32 = core::mem::transmute::<&[u8], &[u32]>(data);

    for i in 0..sector_count {
        wait_for(STATUS_BSY, false);
        wait_for(STATUS_DRQ, true);
        for j in 0..256 {
            io::outl(PORT_DR, data_u32[256*i + j]);
        }
    }
}