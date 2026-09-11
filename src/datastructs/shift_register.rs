/// 128 bit register that stores true or false values
pub struct ShiftRegister {
    register: u128,
}

impl std::fmt::Display for ShiftRegister {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:0128b}", self.register)
    }
}

impl ShiftRegister {
    pub fn new() -> Self {
        return ShiftRegister { register: 0 };
    }

    pub fn push(&mut self, value: bool) {
        self.register <<= 1;
        self.register |= if value { 1 } else { 0 };
    }

    pub fn pop(&mut self) -> bool {
        let result = self.register & 1 == 1;
        self.register >>= 1;
        return result;
    }

    /// Returns an f32 from 0.0 to 1.0 representing how many of the past entries have been true
    pub fn fullness(&self) -> f32 {
        return self.register.count_ones() as f32 / 128.0;
    }

    /// Returns true if the previous n entries have all been true
    pub fn full_recent(&self, n: u32) -> bool{
        let mask = 2_u128.pow(n) - 1;
        return self.register & mask == mask;
    }
}
