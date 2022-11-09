use crate::io;

// I/O addresses for PIC
const MASTER_CMD: u16 = 0x20;
const MASTER_DATA: u16 = 0x21;
const SLAVE_CMD: u16 = 0xA0;
const SLAVE_DATA: u16 = 0xA1;

// Initialization Command Words are given to the PIC in 4 stages.
#[repr(u8)]
#[allow(dead_code)]
enum ICW1 {
    ICW4 = 0x01, // ICW4 needed/not
    Single = 0x02, // single/cascade mode
    Interval4 = 0x04, // call address interval 4/8
    Level = 0x08, // level/edge triggering mode
    Init, // required.
}

#[repr(u8)]
#[allow(dead_code)]
enum ICW4 {
    Mode8086 = 0x01, // 8086/88 mode
    AutoEOI = 0x02, // automatic end of interrupts/not
    BufSlave = 0x08, // buffered mode/slave
    BufMaster = 0x0C, // buffered mode/master
    SFNM = 0x10 // special fully nested/not
}

unsafe fn send(port: u16, bits: u8) {
    io::outb(port, bits);
    io::wait(); // we wait after every instruction to give the PIC time to react on older computers.
}

/// send command to both the master and the slave
unsafe fn send_ms_cmd(bits: u8) {
    send(MASTER_CMD, bits);
    send(SLAVE_CMD, bits);
}

/// send data to both the master and the slave
unsafe fn send_ms_data(bits: u8) {
    send(MASTER_DATA, bits);
    send(SLAVE_DATA, bits);
}

pub fn remap() {
    unsafe {
        // save the PICs' current data (their masks)
        let master_masks: u8 = io::inb(MASTER_DATA);
        let slave_masks: u8 = io::inb(SLAVE_DATA);

        // ICW1
        send_ms_cmd(ICW1::Init as u8 | ICW1::ICW4 as u8);

        // ICW2 (remap the PICs to the end of the exceptions' IRQs)
        send(MASTER_DATA, 0x20);
        send(SLAVE_DATA, 0x28);

        // ICW3
        send(MASTER_DATA, 4); // tells the master that there is a slave PIC at IRQ2 (not 4)
        send(SLAVE_DATA, 2); // tells the slave what its cascade identity is.

        // ICW4
        send_ms_data(ICW4::Mode8086 as u8);

        // restore PICs' masks
        io::outb(MASTER_DATA, master_masks);
        io::outb(SLAVE_DATA, slave_masks);
    }
}

/// if the mask is set for a given IRQ line, the PIC will ignore any requests sent to it.
pub fn set_mask(irq_line: u8, value: bool) {
    assert!(irq_line < 16);

    let port: u16;
    let actual_line: u8;
    if irq_line < 8 {
        port = MASTER_DATA;
        actual_line = irq_line;
    } else {
        port = SLAVE_DATA;
        actual_line = irq_line - 8;
    }

    unsafe {
        let current_masks: u8 = io::inb(port);
        let modify: u8 = 1 << actual_line;
        io::outb(port, if value { current_masks | modify } else { current_masks & !modify });
    }
}
