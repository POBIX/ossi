use crate::{io, pic};

// these ports assume we want to use the master ATA.
/// data register
pub const PORT_DR: u16 = 0x1F0;
/// error register
pub const PORT_ER: u16 = 0x1F1;
/// features register
pub const PORT_FR: u16 = 0x1F1;
/// sector count register
pub const PORT_SCR: u16 = 0x1F2;
/// sector number register / LBA low register
pub const PORT_SNR: u16 = 0x1F3;
/// cylinder low register / LBA mid register
pub const PORT_CLR: u16 = 0x1F4;
/// cylinder high register / LBA high register
pub const PORT_CHR: u16 = 0x1F5;
/// drive / head register
pub const PORT_DHR: u16 = 0x1F6;
/// status register
pub const PORT_SR: u16 = 0x1F7;
/// command register
pub const PORT_CR: u16 = 0x1F7;

// flags for the status register
/// busy executing a command
pub const STATUS_BSY: u8 = 0x80;
/// ready to accept a command
pub const STATUS_RDY: u8 = 0x40;
/// expecting/sending data
pub const STATUS_DRQ: u8 = 0x08;
/// error occurred
pub const STATUS_ERR: u8 = 0x01;

pub fn init() {
    use crate::interrupts::{self, GateType};
    unsafe {
        interrupts::IDT[pic::IRQ_OFFSET + 14] = interrupts::Handler::new(irq14, GateType::DInterrupt, 0);
        interrupts::IDT[pic::IRQ_OFFSET + 15] = interrupts::Handler::new(irq15, GateType::DInterrupt, 0);
    }
    //TODO: query maximum HD size. don't allow going over the sector limit
}

extern "x86-interrupt" fn irq14() {
    pic::send_eoi(14);
}

extern "x86-interrupt" fn irq15() {
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

fn panic_if_error() {
    let error = unsafe { io::inb(PORT_ER) };
    if error != 0 {
        panic!("ATA read failed with error code {:0X}", error);
    }
}

/// reads the first sector_count sectors from the hard disk, at address lba, into buffer.
pub unsafe fn read_sectors(lba: u32, buffer: *mut u8, sector_count: usize) {
    wait_for(STATUS_BSY, false);

    setup_flags(lba, sector_count as u8);
    io::outb(PORT_CR, 0x20); // send the read command
    panic_if_error();

    // the disk sends out 16 bytes at a time.
    let buffer_u16 = buffer as *mut u16;
    const SECTOR_SIZE: usize = 512 / 2; // the sector size in our new unit (u16)

    for i in 0..sector_count {
        wait_for(STATUS_BSY, false);
        wait_for(STATUS_DRQ, true);
        for j in 0..SECTOR_SIZE {
            *buffer_u16.offset((SECTOR_SIZE*i + j) as isize) = io::inw(PORT_DR);
        }
    }
}

/// writes the first sector_count sectors of data to the disk, at address lba.
pub unsafe fn write_sectors(lba: u32, data: *const u8, sector_count: usize) {
    wait_for(STATUS_BSY, false);

    setup_flags(lba, sector_count as u8);
    io::outb(PORT_CR, 0x30); // send the write command
    panic_if_error();

    // the disk receives 32 bytes at a time.
    let data_u32 = data as *const u32;
    const SECTOR_SIZE: usize = 512 / 4; // the sector size in our new unit (u32)

    for i in 0..sector_count {
        wait_for(STATUS_BSY, false);
        wait_for(STATUS_DRQ, true);
        for j in 0..SECTOR_SIZE {
            let block = *data_u32.offset((SECTOR_SIZE*i + j) as isize);
            io::outl(PORT_DR, block);
        }
    }
}
