//! Reconstructed downloaded OFDM services. Register meanings are deliberately
//! descriptive, not guessed RF names. All addresses/constants are ABI facts.
//! ROM services remain ROM calls; their internals are not reconstructed here.
use std::collections::BTreeMap;
pub(super) const ORIGIN: u16 = 0x4980;
pub(super) struct Services {
    pub code: Vec<u8>,
    pub entries: BTreeMap<u16, u16>,
}
struct Asm {
    code: Vec<u8>,
    labels: Vec<Option<usize>>,
    fix: Vec<(usize, usize)>,
    conditions: Vec<usize>,
    last_write: Option<(usize, u16, u8)>,
}
impl Asm {
    fn new() -> Self {
        Self {
            code: vec![],
            labels: vec![],
            fix: vec![],
            conditions: vec![],
            last_write: None,
        }
    }
    fn label(&mut self) -> usize {
        let n = self.labels.len();
        self.labels.push(None);
        n
    }
    fn at(&mut self, l: usize) {
        self.last_write = None;
        assert!(self.labels[l].is_none());
        self.labels[l] = Some(self.code.len());
    }
    fn emit(&mut self, b: &[u8]) {
        self.code.extend_from_slice(b)
    }
    fn jump(&mut self, l: usize) {
        self.emit(&[0x02, 0, 0]);
        self.fix.push((self.code.len() - 2, l));
    }
    fn call_label(&mut self, l: usize) {
        self.emit(&[0x12, 0, 0]);
        self.fix.push((self.code.len() - 2, l));
    }
    fn call(&mut self, a: u16) {
        self.emit(&[0x12, (a >> 8) as u8, a as u8]);
    }
    fn ret(&mut self) {
        self.emit(&[0x22]);
    }
    fn dptr(&mut self, a: u16) {
        self.emit(&[0x90, (a >> 8) as u8, a as u8]);
    }
    fn read(&mut self, a: u16) {
        self.dptr(a);
        self.emit(&[0xE0]);
    }
    fn store(&mut self, a: u16) {
        self.dptr(a);
        self.emit(&[0xF0]);
    }
    fn imm(&mut self, v: u8) {
        if v == 0 {
            self.emit(&[0xE4])
        } else {
            self.emit(&[0x74, v])
        }
    }
    fn write(&mut self, address: u16, value: u8) {
        // Reuse constants only across consecutive writes in one basic block.
        let previous = self
            .last_write
            .filter(|(end, _, _)| *end == self.code.len());
        if previous.map(|(_, _, v)| v) != Some(value) {
            self.imm(value);
        }
        match previous {
            Some((_, a, _)) if a == address => {}
            Some((_, a, _)) if a.checked_add(1) == Some(address) => self.emit(&[0xA3]),
            _ => self.dptr(address),
        }
        self.emit(&[0xF0]);
        self.last_write = Some((self.code.len(), address, value));
    }

    fn copy(&mut self, src: u16, dst: u16) {
        self.read(src);
        self.store(dst);
    }
    fn reg(&mut self, r: u8, v: u8) {
        self.emit(&[0x78 + r, v]);
    }
    fn load_reg(&mut self, a: u16, r: u8) {
        self.read(a);
        self.emit(&[0xF8 + r]);
    }
    fn increment(&mut self, a: u16) {
        self.read(a);
        self.emit(&[0x04, 0xF0]);
    }
    // Long conditional branches cannot silently overflow the 8051 relative range.
    fn zero(&mut self, l: usize) {
        self.conditions.push(self.code.len());
        self.emit(&[0x70, 3]);
        self.jump(l);
    }
    fn nonzero(&mut self, l: usize) {
        self.conditions.push(self.code.len());
        self.emit(&[0x60, 3]);
        self.jump(l);
    }
    fn carry(&mut self, l: usize) {
        self.conditions.push(self.code.len());
        self.emit(&[0x50, 3]);
        self.jump(l);
    }
    fn no_carry(&mut self, l: usize) {
        self.conditions.push(self.code.len());
        self.emit(&[0x40, 3]);
        self.jump(l);
    }
    fn neq(&mut self, a: u16, v: u8, l: usize) {
        self.read(a);
        if v != 0 {
            self.emit(&[0x64, v])
        }
        self.nonzero(l);
    }
    fn eq(&mut self, a: u16, v: u8, l: usize) {
        self.read(a);
        if v != 0 {
            self.emit(&[0x64, v])
        }
        self.zero(l);
    }
    fn writes(&mut self, items: &[(u16, u8)]) {
        for &(a, v) in items {
            self.write(a, v);
        }
    }
    fn finish(mut self, entries: &mut BTreeMap<u16, u16>) -> Vec<u8> {
        // Relax only recorded conditional branches. Handwritten short skips over
        // three-byte jumps stay valid; ordinary LJMPs and LCALLs are not shortened.
        let mut compact = std::collections::BTreeSet::new();
        let remap = |p: usize, set: &std::collections::BTreeSet<usize>| {
            p - 3 * set.iter().filter(|s| **s + 5 <= p).count()
        };
        loop {
            let mut changed = false;
            for &start in &self.conditions {
                if compact.contains(&start) {
                    continue;
                }
                let label = self.fix.iter().find(|(p, _)| *p == start + 3).unwrap().1;
                let target = self.labels[label].unwrap();
                let mut trial = compact.clone();
                trial.insert(start);
                let delta = remap(target, &trial) as isize - remap(start, &trial) as isize - 2;
                if (-128..=127).contains(&delta) {
                    compact = trial;
                    changed = true;
                }
            }
            if !changed {
                break;
            }
        }
        for &(p, l) in &self.fix {
            let target = ORIGIN + remap(self.labels[l].expect("missing label"), &compact) as u16;
            self.code[p..p + 2].copy_from_slice(&target.to_be_bytes());
        }
        for entry in entries.values_mut() {
            *entry = ORIGIN + remap((*entry - ORIGIN) as usize, &compact) as u16;
        }
        let mut result = Vec::new();
        let mut p = 0;
        while p < self.code.len() {
            if compact.contains(&p) {
                let target = u16::from_be_bytes([self.code[p + 3], self.code[p + 4]]) as isize;
                let delta = target - (ORIGIN as isize + result.len() as isize + 2);
                assert!((-128..=127).contains(&delta));
                result.extend_from_slice(&[self.code[p] ^ 0x10, delta as i8 as u8]);
                p += 5;
            } else {
                result.push(self.code[p]);
                p += 1;
            }
        }
        assert!(
            ORIGIN as usize + result.len() <= 0x57CE,
            "services exceed surveyed code RAM: end {:04x}",
            ORIGIN as usize + result.len()
        );
        result
    }
}
fn gain_profile(a: &mut Asm, profile: u16, control: u8, end: usize) {
    a.neq(0x45F6, 1, end);
    a.copy(profile, 0xFB9D);
    a.write(0xEC63, control);
}
fn event(a: &mut Asm) {
    let done = a.label();
    let cases: [usize; 6] = std::array::from_fn(|_| a.label());
    // Decode the reference's arithmetic switch into explicit event identifiers.
    for (id, l) in [0x1A, 0x1B, 0x1D, 0x21, 0x22, 0x65].into_iter().zip(cases) {
        a.emit(&[0xEF, 0x64, id]);
        a.zero(l);
    }
    a.jump(done);
    a.at(cases[0]);
    a.writes(&[(0x4478, 0), (0xF076, 0)]);
    let gain_done = a.label();
    gain_profile(a, 0x45FB, 0x29, gain_done);
    a.at(gain_done);
    a.writes(&[(0x457F, 0), (0x449E, 0), (0x4584, 0), (0x4585, 255)]);
    a.copy(0x4571, 0x4570);
    a.jump(done);
    a.at(cases[1]);
    a.writes(&[(0x4478, 0), (0xF076, 0), (0x449E, 0)]);
    let gain_done = a.label();
    gain_profile(a, 0x45FB, 0x29, gain_done);
    a.at(gain_done);
    let increment = a.label();
    a.load_reg(0x4571, 7);
    a.read(0x4570);
    a.emit(&[0x6F]);
    a.nonzero(increment);
    a.writes(&[
        (0x4570, 0),
        (0x4465, 0),
        (0x4466, 0),
        (0x44E3, 0),
        (0x44E9, 0),
        (0x43A2, 0),
    ]);
    a.neq(0x45BA, 0, done);
    a.neq(0x44E2, 0x11, done);
    a.neq(0x446D, 1, done);
    a.call(0x6713);
    a.jump(done);
    a.at(increment);
    a.increment(0x4570);
    a.jump(done);
    a.at(cases[2]);
    a.writes(&[(0xF0EB, 1), (0xF0EB, 0)]);
    a.jump(done);
    a.at(cases[3]);
    a.neq(0x4466, 1, done);
    a.writes(&[(0xF110, 0), (0xF10A, 1)]);
    a.jump(done);
    a.at(cases[4]);
    a.neq(0x45FD, 0, done);
    a.eq(0xF985, 0, done);
    let clear = a.label();
    a.neq(0xF9D9, 3, clear);
    a.write(0xF98F, 1);
    a.jump(done);
    a.at(clear);
    a.write(0xF98F, 0);
    a.jump(done);
    a.at(cases[5]);
    a.neq(0xF5B0, 1, done);
    a.neq(0x42DE, 1, done);
    a.writes(&[(0x42DE, 0), (0x45FF, 1)]);
    a.at(done);
    a.ret();
}
fn receiver_state(a: &mut Asm) {
    a.call(0x67B2);
    a.call(0x6752);
    let selected = a.label();
    a.neq(0x44E2, 0x11, selected);
    a.writes(&[(0xF249, 0), (0xF261, 0), (0x4586, 1)]);
    a.at(selected);
    let final_state = a.label();
    let active = a.label();
    let mid = a.label();
    let phase2 = a.label();
    let mode_b = a.label();
    let mode_d = a.label();
    a.neq(0x4465, 0, active);
    a.neq(0x4466, 0, mid);
    a.neq(0x44E3, 0, mid);
    a.call(0x673A);
    gain_profile(a, 0x45F7, 0x29, final_state);
    a.jump(final_state);
    a.at(mid);
    a.neq(0x44E9, 0, phase2);
    a.neq(0x4466, 0, mode_b);
    a.neq(0x44E3, 1, mode_b);
    a.call(0x6743);
    gain_profile(a, 0x45F8, 0x09, final_state);
    a.jump(final_state);
    a.at(mode_b);
    a.call(0x6740);
    gain_profile(a, 0x45F9, 0x29, final_state);
    a.jump(final_state);
    a.at(phase2);
    a.neq(0x4466, 0, mode_d);
    a.neq(0x44E3, 1, mode_d);
    a.call(0x6749);
    gain_profile(a, 0x45F8, 0x09, final_state);
    a.jump(final_state);
    a.at(mode_d);
    a.call(0x6746);
    gain_profile(a, 0x45F9, 0x29, final_state);
    a.jump(final_state);
    a.at(active);
    a.call(0x673D);
    gain_profile(a, 0x45FA, 0x29, final_state);
    a.at(final_state);
    a.copy(0x455F, 0xF029);
    a.write(0x4586, 0);
    a.copy(0x4584, 0x4585);
    a.copy(0x4587, 0x458D);
    a.ret();
}
fn proximity(a: &mut Asm) {
    let outside = a.label();
    a.load_reg(0x4588, 7);
    a.read(0x45EC);
    a.emit(&[0xC3, 0x9F, 0xC3, 0x94, 5]);
    a.no_carry(outside);
    a.write(0x459B, 1);
    a.ret();
    a.at(outside);
    a.write(0x459B, 0);
    a.ret();
}
fn threshold_tables(a: &mut Asm) {
    let data = a.label();
    let selected = a.label();
    let done = a.label();
    let again = a.label();
    a.emit(&[0xEF, 0x14, 0xFC, 0xC3, 0x94, 3]);
    a.no_carry(selected);
    a.emit(&[0xEC, 0x75, 0xF0, 14, 0xA4, 0xFC]);
    a.reg(6, 0);
    a.at(again);
    for (offset, destination) in [(0, 0x95), (7, 0x9C)] {
        a.emit(&[0xEE, 0x2C]);
        if offset != 0 {
            a.emit(&[0x24, offset]);
        }
        a.emit(&[0x90, 0, 0]);
        a.fix.push((a.code.len() - 2, data));
        a.emit(&[
            0x93,
            0xFD,
            0xEE,
            0x24,
            destination,
            0xF5,
            0x82,
            0x75,
            0x83,
            0x41,
            0xED,
            0xF0,
        ]);
    }
    a.emit(&[0x0E, 0xBE, 7, 3]);
    a.jump(selected);
    a.jump(again);
    a.at(selected);
    a.eq(0x45FC, 0, done);
    a.neq(0xF903, 1, done);
    a.neq(0xF902, 1, done);
    a.writes(&[(0x4195, 0xEA), (0x419C, 0)]);
    a.at(done);
    a.ret();
    a.at(data);
    for values in [
        [2250u16, 2634, 3274, 4170, 5322, 6730, 8394],
        [128, 512, 800, 1000, 1200, 1500, 2100],
        [125, 180, 200, 250, 300, 850, 2100],
    ] {
        for v in values {
            a.emit(&[v as u8]);
        }
        for v in values {
            a.emit(&[(v >> 8) as u8]);
        }
    }
}

fn serial_probe(a: &mut Asm) {
    a.writes(&[(0xF1A3, 0), (0xF1A4, 0), (0xF1A5, 0), (0x4610, 0)]);
    for value in [0x78, 0x9A] {
        a.emit(&[0x75, 0x99, value]);
        a.reg(7, 1);
        a.reg(6, 0x15);
        a.call(0xE3CC);
        let next = a.label();
        a.emit(&[0xE5, 0x99, 0x64, value]);
        a.nonzero(next);
        a.write(0x4610, 1);
        a.at(next);
    }
    let failed = a.label();
    let notify = a.label();
    a.neq(0x4610, 0, failed);
    a.call(0x6761);
    a.jump(notify);
    a.at(failed);
    a.write(0x4319, 0);
    a.at(notify);
    a.reg(7, 0x4F);
    a.call(0x6680);
    a.ret();
}
fn clear_receiver_ram(a: &mut Asm) {
    // Inclusive end 0x4611: the reference uses SETB C before subtracting the end.
    a.dptr(0x4422);
    a.imm(0);
    let again = a.label();
    a.at(again);
    a.emit(&[0xF0, 0xA3]);
    a.emit(&[0xE5, 0x83, 0x64, 0x46]);
    let more = a.label();
    a.nonzero(more);
    a.emit(&[0xE5, 0x82, 0x64, 0x12]);
    let done = a.label();
    a.zero(done);
    a.at(more);
    a.imm(0);
    a.jump(again);
    a.at(done);
    a.ret();
}
fn fixed_defaults(a: &mut Asm) {
    // The original unsigned-byte < 0 test is unreachable, but retain its MMIO read.
    a.read(0x4322);
    for (i, v) in [9, 0x75, 0x0C, 0xF5, 0x35, 0x40].into_iter().enumerate() {
        a.write(0x42FD + i as u16, v);
    }
    a.ret();
}
fn load_long(a: &mut Asm, base: u16, first: u8) {
    a.dptr(base);
    for i in 0..4 {
        if i != 0 {
            a.emit(&[0xA3]);
        }
        a.emit(&[0xE0, 0xF8 + first + i]);
    }
}
fn correction_pair(a: &mut Asm, lo: u16, hi: u16, out: u16, wrap: usize) {
    a.writes(&[(0x421C, 0), (0x421D, 0)]);
    a.copy(hi, 0x421E);
    a.copy(lo, 0x421F);
    a.call_label(wrap);
    a.copy(0x421F, out);
    a.copy(0x421E, out + 1);
}
fn wrap_correction(a: &mut Asm) {
    load_long(a, 0x421C, 4);
    load_long(a, 0x4232, 0);
    // Add a 32-bit ROM-maintained offset; no modulo shortcut (one wrap only).
    a.emit(&[
        0xEF, 0x2B, 0xFF, 0xEE, 0x3A, 0xFE, 0xED, 0x39, 0xFD, 0xEC, 0x38, 0xFC,
    ]);
    a.dptr(0x421C);
    a.call(0x3B04);
    a.reg(4, 0);
    a.reg(5, 0);
    a.reg(6, 0x40);
    a.reg(7, 0);
    load_long(a, 0x421C, 0);
    a.emit(&[0xC3]);
    a.call(0x3B46);
    let negative = a.label();
    let store = a.label();
    a.carry(negative);
    load_long(a, 0x421C, 4);
    a.emit(&[
        0xEF, 0x24, 0, 0xFF, 0xEE, 0x34, 0xC0, 0xFE, 0xED, 0x34, 0xFF, 0xFD, 0xEC, 0x34, 0xFF, 0xFC,
    ]);
    a.dptr(0x421C);
    a.call(0x3B04);
    a.jump(store);
    a.at(negative);
    for r in 4..8 {
        a.reg(r, 0);
    }
    load_long(a, 0x421C, 0);
    a.emit(&[0xC3]);
    a.call(0x3B46);
    a.no_carry(store);
    load_long(a, 0x421C, 4);
    a.emit(&[
        0xEF, 0x24, 0, 0xFF, 0xEE, 0x34, 0x40, 0xFE, 0xED, 0x34, 0, 0xFD, 0xEC, 0x34, 0, 0xFC,
    ]);
    a.dptr(0x421C);
    a.call(0x3B04);
    a.at(store);
    a.ret();
}
fn apply_corrections(a: &mut Asm, wrap: usize) {
    let active = a.label();
    a.neq(0x4466, 0, active);
    a.write(0x42FC, 0);
    a.ret();
    a.at(active);
    let done = a.label();
    a.neq(0x42AD, 1, done);
    a.write(0x42AD, 0);
    correction_pair(a, 0x45F0, 0x45F1, 0xF10E, wrap);
    correction_pair(a, 0x45F4, 0x45F5, 0xF112, wrap);
    for (src, dst) in [
        (0x45EF, 0xF109),
        (0x45EE, 0xF110),
        (0x45F3, 0xF10B),
        (0x45F2, 0xF114),
    ] {
        a.copy(src, dst);
    }
    a.write(0xF10A, 1);
    a.at(done);
    a.write(0xEC56, 1);
    a.ret();
}
fn tracking_profile(a: &mut Asm) {
    a.writes(&[
        (0xF12F, 0),
        (0xF134, 1),
        (0xF137, 0x10),
        (0xF138, 6),
        (0xF13B, 0x60),
        (0xF13C, 0x1A),
        (0xF13F, 0xC0),
        (0xF140, 0x2D),
        (0xF133, 1),
        (0xF136, 0x0D),
        (0xF139, 0x10),
        (0xF13A, 0x18),
        (0xF13D, 0x60),
        (0xF13E, 0x1B),
        (0xF141, 0x40),
        (0xF142, 0x3A),
        (0xF135, 1),
        (0xF111, 1),
    ]);
    a.ret();
}
fn measure(a: &mut Asm, timeout: usize) {
    // The original spins forever. Keep a bounded poll and return R7=0xFF on fault.
    // Stack-save R6/R7 so interrupts/ROM callers don't see a new persistent scratch.
    a.emit(&[0xC0, 6, 0xC0, 7]);
    a.reg(6, 255);
    a.reg(7, 255);
    let poll = a.label();
    let ready = a.label();
    a.at(poll);
    a.eq(0xF62F, 1, ready);
    a.emit(&[0xDF, 3]);
    let decrement = a.label();
    a.jump(decrement);
    a.jump(poll);
    a.at(decrement);
    a.emit(&[0xDE, 3]);
    let failed = a.label();
    a.jump(failed);
    a.jump(poll);
    a.at(failed);
    a.emit(&[0xD0, 7, 0xD0, 6]);
    a.jump(timeout);
    a.at(ready);
    a.emit(&[0xD0, 7, 0xD0, 6]);
    a.copy(0xF632, 0x42BE);
    a.copy(0xF633, 0x42BD);
}
fn below_threshold(a: &mut Asm) {
    a.load_reg(0x4682, 7);
    a.load_reg(0x4681, 6);
    a.read(0x42BE);
    a.emit(&[0xC3, 0x9F]);
    a.read(0x42BD);
    a.emit(&[0x9E]);
}
fn acquisition(a: &mut Asm, profile: usize) {
    let timeout = a.label();
    let finish = a.label();
    let exit = a.label();
    a.writes(&[(0x42BC, 0), (0x42AF, 0)]);
    a.copy(0x44B2, 0x4682);
    a.copy(0x44B3, 0x4681);
    a.write(0xF625, 0);
    a.copy(0x44B1, 0xF626);
    a.writes(&[
        (0xF62B, 2),
        (0xF62C, 2),
        (0xF62D, 0),
        (0xF625, 1),
        (0xF62F, 1),
    ]);
    measure(a, timeout);
    a.copy(0xF633, 0x44C1);
    a.copy(0xF632, 0x44C2);
    let threshold = a.label();
    a.neq(0x4466, 1, threshold);
    a.copy(0x45E9, 0x4681);
    a.at(threshold);
    below_threshold(a);
    a.no_carry(finish);
    let search = a.label();
    let found = a.label();
    let next_gain = a.label();
    let trigger = a.label();
    a.at(search);
    below_threshold(a);
    a.no_carry(found);
    a.neq(0xF62C, 2, next_gain);
    a.write(0xF62C, 1);
    a.jump(trigger);
    a.at(next_gain);
    a.increment(0xF62B);
    a.write(0xF62C, 2);
    a.at(trigger);
    a.write(0xF62F, 1);
    measure(a, timeout);
    a.neq(0xF62B, 15, search);
    a.at(found);
    a.copy(0xF62B, 0x44B9);
    a.load_reg(0x44B4, 7);
    a.read(0xF62B);
    a.emit(&[0xC3, 0x9F]);
    let not_tracking = a.label();
    let selected = a.label();
    a.carry(not_tracking);
    a.write(0x42AD, 1);
    a.call_label(profile);
    a.jump(selected);
    a.at(not_tracking);
    a.write(0x42AD, 0);
    a.at(selected);
    a.read(0xF62B);
    a.emit(&[0x14, 0xF0]);
    a.reg(7, 6);
    a.call(0x67FA);
    a.write(0xF62D, 1);
    a.call(0xB4EF);
    a.neq(0x44CD, 0, exit);
    for i in 0..4 {
        a.copy(0xF64C + i, 0x4684 + i);
    }
    let duplicate = a.label();
    let duplicates_done = a.label();
    for i in 0..4 {
        for j in i + 1..4 {
            a.load_reg(0x4684 + i, 7);
            a.read(0x4684 + j);
            a.emit(&[0x6F]);
            a.zero(duplicate);
        }
    }
    a.jump(duplicates_done);
    a.at(duplicate);
    a.write(0x42BC, 1);
    a.at(duplicates_done);
    a.write(0x4683, 4);
    let pass = a.label();
    a.at(pass);
    a.read(0x4683);
    a.emit(&[0xC3, 0x94, 6]);
    a.no_carry(finish);
    for i in 0..4 {
        let next = a.label();
        let measured = a.label();
        let invoke = a.label();
        a.load_reg(0x4683, 7);
        a.read(0xF64C + i);
        a.emit(&[0x6F]);
        a.nonzero(next);
        a.neq(0x42BC, 1, measured);
        a.neq(0x4683, 5, measured);
        a.writes(&[(0x42B1, 0xA0), (0x42B0, 15), (0x42B3, 0xEF), (0x42B2, 0x15)]);
        a.jump(invoke);
        a.at(measured);
        for (k, dst) in [0x42B1, 0x42B0, 0x42B3, 0x42B2].into_iter().enumerate() {
            a.copy(0xF634 + i * 6 + k as u16, dst);
        }
        a.at(invoke);
        a.call(0x9148);
        a.at(next);
    }
    a.increment(0x4683);
    a.jump(pass);
    a.at(finish);
    a.write(0xF625, 0);
    for (src, dst) in [
        (0xF109, 0x45EF),
        (0xF10E, 0x45F0),
        (0xF10F, 0x45F1),
        (0xF110, 0x45EE),
        (0xF10B, 0x45F3),
        (0xF112, 0x45F4),
        (0xF113, 0x45F5),
        (0xF114, 0x45F2),
    ] {
        a.copy(src, dst);
    }
    let after_profile = a.label();
    a.neq(0x4466, 1, after_profile);
    a.write(0x42AD, 1);
    a.call_label(profile);
    a.at(after_profile);
    let notify = a.label();
    a.neq(0x42AD, 0, notify);
    a.neq(0x42AF, 1, notify);
    a.write(0xF10A, 1);
    a.at(notify);
    a.reg(7, 1);
    a.call(0xB10B);
    a.reg(7, 0x10);
    a.call(0x6680);
    a.at(exit);
    a.ret();
    a.at(timeout);
    a.write(0xF625, 0);
    a.reg(7, 255);
    a.ret();
}
fn timer(a: &mut Asm) {
    let incremented = a.label();
    a.increment(0x441E);
    a.nonzero(incremented);
    a.increment(0x441D);
    a.at(incremented);
    let heartbeat = a.label();
    a.eq(0xF001, 0, heartbeat);
    a.reg(7, 1);
    a.call(0x97F3);
    a.at(heartbeat);
    a.load_reg(0x441D, 6);
    a.load_reg(0x441E, 7);
    a.reg(4, 0);
    a.reg(5, 10);
    a.call(0x3B70);
    a.emit(&[0xED, 0x4C]);
    let flags = a.label();
    a.nonzero(flags);
    a.eq(0x4320, 0, flags);
    a.call(0x668C);
    a.call(0x6695);
    a.at(flags);
    let present = a.label();
    let present_done = a.label();
    for addr in [0xF9F1, 0xF9F2, 0xF9F3] {
        a.neq(addr, 0, present);
    }
    a.write(0x445F, 0);
    a.jump(present_done);
    a.at(present);
    a.write(0x445F, 1);
    a.at(present_done);
    let mode0 = a.label();
    let mode2 = a.label();
    let periodic = a.label();
    let add_gate = a.label();
    a.neq(0x4609, 1, mode0);
    a.write(0x4680, 4);
    a.jump(add_gate);
    a.at(mode0);
    a.neq(0x4609, 0, mode2);
    a.write(0xF862, 15);
    a.jump(periodic);
    a.at(mode2);
    a.neq(0x4609, 2, periodic);
    a.read(0xF9F2);
    a.emit(&[0x25, 0xE0, 0xFF]);
    a.read(0xF9F1);
    a.emit(&[0x25, 0xE0, 0x25, 0xE0, 0x4F, 0xFF]);
    a.read(0xF9F3);
    a.emit(&[0x4F]);
    a.store(0x4680);
    a.at(add_gate);
    let gate_done = a.label();
    a.eq(0x460F, 0, gate_done);
    a.read(0x4680);
    a.emit(&[0x24, 8, 0xF0]);
    a.at(gate_done);
    a.copy(0x4680, 0xF862);
    a.at(periodic);
    a.load_reg(0x441D, 6);
    a.load_reg(0x441E, 7);
    a.reg(4, 0);
    a.reg(5, 50);
    a.call(0x3B70);
    a.emit(&[0xED, 0x4C]);
    let done = a.label();
    a.nonzero(done);
    a.call(0xDF34);
    a.call(0xDB03);
    a.at(done);
    a.emit(&[0xC2, 0xCF]);
    a.ret();
}
fn layer_parameters(a: &mut Asm) {
    a.write(0x45FC, 1);
    for (index, address) in [(0, 0xF903), (1, 0xF907), (2, 0xF90B)] {
        if index == 1 {
            a.write(0x45FC, 0);
        }
        a.load_reg(address, 7);
        a.reg(5, index);
        a.call(0x303A);
    }
    a.ret();
}
fn layer_weight(a: &mut Asm, index: u8, address: u16, add: bool) {
    a.reg(7, index);
    a.call(0xDC8D);
    a.load_reg(address, 7);
    a.read(0x43EC);
    a.emit(&[0x8F, 0xF0, 0xA4, 0xFF, 0xAE, 0xF0]);
    if add {
        a.read(0x43EE);
        a.emit(&[0x2F, 0xF0]);
        a.read(0x43ED);
        a.emit(&[0x3E, 0xF0]);
    } else {
        a.emit(&[0xEE]);
        a.store(0x43ED);
        a.emit(&[0xEF]);
        a.store(0x43EE);
    }
}
fn quality(a: &mut Asm) {
    layer_weight(a, 1, 0xF906, false);
    for (index, address) in [(2, 0xF90A), (3, 0xF90E)] {
        let next = a.label();
        a.eq(address, 15, next);
        layer_weight(a, index, address, true);
        a.at(next);
    }
    a.load_reg(0x43ED, 6);
    a.load_reg(0x43EE, 7);
    a.reg(4, 0);
    a.reg(5, 13);
    a.call(0x3B70);
    a.emit(&[0xEF]);
    a.store(0x446B);
    let done = a.label();
    let zero = a.label();
    let middle = a.label();
    let high = a.label();
    a.neq(0x4609, 1, done);
    a.read(0x44C3);
    a.emit(&[0xC3, 0x94, 50]);
    a.carry(zero);
    a.eq(0xF9F1, 0, zero);
    a.read(0x44C3);
    a.emit(&[0xC3, 0x94, 58]);
    a.no_carry(middle);
    a.write(0x446B, 32);
    a.jump(done);
    a.at(middle);
    a.read(0x44C3);
    a.emit(&[0xC3, 0x94, 76]);
    a.no_carry(high);
    a.read(0x44C3);
    a.emit(&[0x24, 206, 0x25, 0xE0, 0x25, 0xE0]);
    a.store(0x446B);
    a.jump(done);
    a.at(high);
    a.write(0x446B, 100);
    a.jump(done);
    a.at(zero);
    a.write(0x446B, 0);
    a.at(done);
    a.ret();
}
fn zero_long(a: &mut Asm, address: u16) {
    a.dptr(address);
    a.imm(0);
    for i in 0..4 {
        if i != 0 {
            a.emit(&[0xA3]);
        }
        a.emit(&[0xF0]);
    }
}
fn subtract_from_0x800000(a: &mut Asm, address: u16) {
    load_long(a, address, 0);
    a.emit(&[
        0xC3, 0xE4, 0x9B, 0xFF, 0xE4, 0x9A, 0xFE, 0x74, 0x80, 0x99, 0xFD, 0xE4, 0x98, 0xFC,
    ]);
    a.dptr(address);
    a.call(0x3B04);
}
fn bounded_ready(a: &mut Asm, address: u16, gate: bool, timeout: usize) {
    a.emit(&[0xC0, 6, 0xC0, 7]);
    a.reg(6, 255);
    a.reg(7, 255);
    let poll = a.label();
    let ready = a.label();
    a.at(poll);
    if gate {
        a.neq(0x4463, 1, ready);
    }
    a.eq(address, 1, ready);
    a.emit(&[0xDF, 3]);
    let decrement = a.label();
    a.jump(decrement);
    a.jump(poll);
    a.at(decrement);
    a.emit(&[0xDE, 3]);
    let fail = a.label();
    a.jump(fail);
    a.jump(poll);
    a.at(fail);
    a.emit(&[0xD0, 7, 0xD0, 6]);
    a.jump(timeout);
    a.at(ready);
    a.emit(&[0xD0, 7, 0xD0, 6]);
}
fn ready_flag(a: &mut Asm, address: u16) {
    let no = a.label();
    let done = a.label();
    a.neq(address, 1, no);
    a.write(0x423B, 1);
    a.jump(done);
    a.at(no);
    a.write(0x423B, 0);
    a.at(done);
}
fn signed_search(a: &mut Asm) {
    let timeout = a.label();
    let done = a.label();
    a.write(0x45FF, 0);
    zero_long(a, 0x4228);
    zero_long(a, 0x422C);
    a.writes(&[
        (0x4230, 0),
        (0x4231, 0),
        (0xF5B3, 1),
        (0xF5B1, 0),
        (0xF5B0, 0),
        (0xF5C2, 1),
        (0xF5B0, 1),
        (0xF5C2, 0),
        (0x4238, 0),
    ]);
    let lanes = a.label();
    a.neq(0x421B, 0, lanes);
    a.write(0x4238, 1);
    a.at(lanes);
    zero_long(a, 0x4224);
    a.write(0x423C, 0);
    let outer = a.label();
    let finish = a.label();
    let inner = a.label();
    let next_lane = a.label();
    let increment = a.label();
    a.at(outer);
    a.load_reg(0x4238, 7);
    a.read(0x423C);
    a.emit(&[0xD3, 0x9F]);
    a.no_carry(finish);
    a.copy(0x423C, 0xF5B8);
    a.load_reg(0x4237, 7);
    a.load_reg(0x4236, 6);
    a.emit(&[0xC3, 0xE4, 0x9F, 0xFF, 0xE4, 0x9E]);
    a.store(0x4239);
    a.emit(&[0xEF]);
    a.store(0x423A);
    a.at(inner);
    a.load_reg(0x4237, 7);
    a.load_reg(0x4236, 6);
    a.load_reg(0x4239, 4);
    a.read(0x423A);
    a.emit(&[0xD3, 0x9F, 0xEE, 0x64, 0x80, 0xF8, 0xEC, 0x64, 0x80, 0x98]);
    a.no_carry(next_lane);
    a.neq(0x45FF, 0, increment);
    a.copy(0x423A, 0xF411);
    a.copy(0x4239, 0xF412);
    a.write(0xF5B5, 1);
    bounded_ready(a, 0xF5B5, true, timeout);
    ready_flag(a, 0xF5B5);
    a.load_reg(0x4239, 6);
    a.load_reg(0x423A, 7);
    a.load_reg(0x423C, 5);
    a.call(0x721F);
    a.at(increment);
    a.increment(0x423A);
    a.nonzero(inner);
    a.increment(0x4239);
    a.jump(inner);
    a.at(next_lane);
    a.increment(0x423C);
    a.jump(outer);
    a.at(finish);
    load_long(a, 0x4228, 4);
    load_long(a, 0x422C, 0);
    a.emit(&[0xD3]);
    a.call(0x3B46);
    let no_flip = a.label();
    a.carry(no_flip);
    a.read(0x4477);
    a.emit(&[0x54, 1]);
    a.zero(no_flip);
    a.read(0x4478);
    a.emit(&[0x64, 1, 0xF0]);
    a.store(0xF076);
    a.write(0x4232, 0);
    for (src, dst) in [(0xF1B5, 0x4233), (0xF1B4, 0x4234), (0xF1B3, 0x4235)] {
        a.copy(src, dst);
    }
    a.reg(4, 0);
    a.reg(5, 0x40);
    a.reg(6, 0);
    a.reg(7, 0);
    load_long(a, 0x4232, 0);
    a.emit(&[0xC3]);
    a.call(0x3B46);
    let negate = a.label();
    let phase = a.label();
    a.carry(negate);
    subtract_from_0x800000(a, 0x4232);
    a.jump(phase);
    a.at(negate);
    load_long(a, 0x4232, 4);
    a.call(0x3AB2);
    a.dptr(0x4232);
    a.call(0x3B04);
    a.at(phase);
    subtract_from_0x800000(a, 0x421C);
    a.writes(&[(0xF1EB, 1), (0xF1EB, 0)]);
    for (src, dst) in [(0x4233, 0xF1A5), (0x4234, 0xF1A4), (0x4235, 0xF1A3)] {
        a.copy(src, dst);
    }
    let complement = a.label();
    a.neq(0xF114, 0, complement);
    a.eq(0xF110, 0, no_flip);
    a.at(complement);
    for address in [0xF10E, 0xF10F, 0xF112, 0xF113] {
        a.read(address);
        a.emit(&[0xF4, 0xF0]);
    }
    a.write(0xF10A, 1);
    a.at(no_flip);
    a.copy(0x4230, 0xF40D);
    a.copy(0x4231, 0xF40C);
    a.write(0xF5B6, 1);
    bounded_ready(a, 0xF5B7, true, timeout);
    ready_flag(a, 0xF5B7);
    bounded_ready(a, 0x423B, false, timeout);
    a.writes(&[(0xF5B6, 0), (0xF5C2, 1), (0xF5B0, 0), (0xF5C2, 0)]);
    a.neq(0x45FF, 1, done);
    a.writes(&[(0x45FF, 0), (0x42DE, 1)]);
    a.call(0xAA9D);
    a.emit(&[0x75, 0x23, 0]);
    a.reg(3, 1);
    a.reg(2, 0x42);
    a.reg(1, 0xDF);
    a.reg(5, 1);
    a.reg(4, 0);
    a.call(0x3D2C);
    a.at(done);
    a.ret();
    a.at(timeout);
    a.writes(&[(0xF5B6, 0), (0xF5C2, 1), (0xF5B0, 0), (0xF5C2, 0)]);
    a.reg(7, 255);
    a.ret();
}

pub(super) fn build() -> Services {
    let mut a = Asm::new();
    let mut entries = BTreeMap::new();
    let profile = a.label();
    let wrap = a.label();
    for (reference, emit) in [
        (0x4DDA, event as fn(&mut Asm)),
        (0x503F, receiver_state),
        (0x4EF6, proximity),
        (0x4F13, threshold_tables),
        (0x554E, serial_probe),
        (0x559B, clear_receiver_ram),
        (0x578D, fixed_defaults),
        (0x4A1B, timer),
        (0x4954, quality),
        (0x4AF1, layer_parameters),
        (0x4B1B, signed_search),
    ] {
        entries.insert(reference, ORIGIN + a.code.len() as u16);
        emit(&mut a);
    }
    entries.insert(0x55B6, ORIGIN + a.code.len() as u16);
    apply_corrections(&mut a, wrap);
    entries.insert(0x5153, ORIGIN + a.code.len() as u16);
    acquisition(&mut a, profile);
    a.at(profile);
    tracking_profile(&mut a);
    a.at(wrap);
    wrap_correction(&mut a);
    Services {
        code: a.finish(&mut entries),
        entries,
    }
}
