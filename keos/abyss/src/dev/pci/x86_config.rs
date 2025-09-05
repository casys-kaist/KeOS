use crate::{
    spinlock::{SpinLock, SpinLockGuard},
    x86_64::pio::Pio,
};

#[derive(Clone, Copy)]
pub struct X86Config;
impl X86Config {
    fn lock() -> SpinLockGuard<'static, ()> {
        static PCI_LOCK: SpinLock<()> = SpinLock::new(());
        PCI_LOCK.lock()
    }
    #[inline]
    pub fn make_address(&self, bus: u8, slot: u8, func: u8, offset: u8) -> usize {
        0x80000000
            | ((bus as usize) << 16)
            | ((slot as usize) << 11)
            | ((func as usize) << 8)
            | (offset as usize)
    }

    pub fn write_u8(&self, addr: usize, v: u8) {
        let _guard = Self::lock();
        let (addr, shift) = (addr & !3, (addr & 3) * 8);
        let mask = !(0xff << shift);
        assert!(shift <= 24);
        Pio::new(0xCF8).write_u32(addr as u32);
        let vv = (Pio::new(0xCFC).read_u32() & mask) | ((v as u32) << shift);
        Pio::new(0xCFC).write_u32(vv);
        _guard.unlock();
    }
    pub fn write_u16(&self, addr: usize, v: u16) {
        let _guard = Self::lock();
        let (addr, shift) = (addr & !3, (addr & 3) * 8);
        let mask = !(0xffff << shift);
        assert!(shift <= 16);
        Pio::new(0xCF8).write_u32(addr as u32);
        let vv = (Pio::new(0xCFC).read_u32() & mask) | ((v as u32) << shift);
        Pio::new(0xCFC).write_u32(vv);
        _guard.unlock();
    }
    pub fn write_u32(&self, addr: usize, v: u32) {
        let (addr, shift) = (addr & !3, (addr & 3) * 8);
        assert!(shift == 0);
        let _guard = Self::lock();
        Pio::new(0xCF8).write_u32(addr as u32);
        Pio::new(0xCFC).write_u32(v);
        _guard.unlock();
    }
    pub fn read_u8(&self, addr: usize) -> u8 {
        let _guard = Self::lock();
        let (addr, shift) = (addr & !3, (addr & 3) * 8);
        assert!(shift <= 24);
        Pio::new(0xCF8).write_u32(addr as u32);
        let val = (Pio::new(0xCFC).read_u32() >> shift) as u8;
        _guard.unlock();
        val
    }
    pub fn read_u16(&self, addr: usize) -> u16 {
        let _guard = Self::lock();
        let (addr, shift) = (addr & !3, (addr & 3) * 8);
        assert!(shift <= 16);
        Pio::new(0xCF8).write_u32(addr as u32);
        let val = (Pio::new(0xCFC).read_u32() >> shift) as u16;
        _guard.unlock();
        val
    }
    pub fn read_u32(&self, addr: usize) -> u32 {
        let _guard = Self::lock();
        let (addr, shift) = (addr & !3, (addr & 3) * 8);
        assert!(shift == 0);
        Pio::new(0xCF8).write_u32(addr as u32);
        let val = Pio::new(0xCFC).read_u32();
        _guard.unlock();
        val
    }
}
