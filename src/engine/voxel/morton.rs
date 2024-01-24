pub type MortonCode = u32;

pub mod util {
    use super::MortonCode;

    pub fn morton_encode(x: u32, y: u32, z: u32) -> MortonCode {
        let mut answer = 0;
        for i in 0..10 {
            answer |= ((x & (1 << i)) << 2 * i)
                | ((y & (1 << i)) << (2 * i + 1))
                | ((z & (1 << i)) << (2 * i + 2));
        }
        answer
    }

    pub fn morton_decode(code: MortonCode) -> (u32, u32, u32) {
        let mut x = 0;
        let mut y = 0;
        let mut z = 0;
        for i in 0..10 {
            x |= (code & (1 << (3 * i))) >> (2 * i);
            y |= (code & (1 << (3 * i + 1))) >> (2 * i + 1);
            z |= (code & (1 << (3 * i + 2))) >> (2 * i + 2);
        }
        (x, y, z)
    }
}
