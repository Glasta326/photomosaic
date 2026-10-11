/// 128 bit register that stores true or false values
pub struct ShiftRegister128 {
    register: u128,
}

/// 64 bit reigster that stores true or false values
pub struct ShiftRegister64 {
    register: u64,
}

impl std::fmt::Display for ShiftRegister128 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:0128b}", self.register)
    }
}

impl std::fmt::Display for ShiftRegister64 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:064b}", self.register)
    }
}

impl ShiftRegister128 {
    pub fn new() -> Self {
        return ShiftRegister128 { register: 0 };
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

impl ShiftRegister64 {
    pub fn new() -> Self {
        return ShiftRegister64 { register: 0 };
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
        return self.register.count_ones() as f32 / 64.0;
    }

    /// Returns true if the previous n entries have all been true
    pub fn full_recent(&self, n: u32) -> bool{
        let mask = 2_u64.pow(n) - 1;
        return self.register & mask == mask;
    }
}
