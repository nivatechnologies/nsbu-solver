//! Compile-time degree-three three-variable indices and all 84 admissible products.
pub(super) const COUNT: usize = 20;
pub(super) const POWERS: [[u8; 3]; COUNT] = powers();
pub(super) const PRODUCTS: [[u8; 3]; 84] = products();
pub(super) const DERIVATIVES: [[[u8; 3]; 10]; 3] = derivatives();
const fn powers() -> [[u8; 3]; COUNT] {
    let mut result = [[0; 3]; COUNT];
    let mut index = 0;
    let mut degree = 0;
    while degree <= 3 {
        let mut a = 0;
        while a <= degree {
            let mut b = 0;
            while b <= degree - a {
                result[index] = [a, b, degree - a - b];
                index += 1;
                b += 1;
            }
            a += 1;
        }
        degree += 1;
    }
    result
}
const fn find(power: [u8; 3]) -> u8 {
    let mut index = 0;
    while index < COUNT {
        let value = POWERS[index];
        if value[0] == power[0] && value[1] == power[1] && value[2] == power[2] {
            return index as u8;
        }
        index += 1;
    }
    COUNT as u8
}
const fn products() -> [[u8; 3]; 84] {
    let mut result = [[0; 3]; 84];
    let mut count = 0;
    let mut left = 0;
    while left < COUNT {
        let mut right = 0;
        while right < COUNT {
            let a = POWERS[left];
            let b = POWERS[right];
            let power = [a[0] + b[0], a[1] + b[1], a[2] + b[2]];
            if power[0] + power[1] + power[2] <= 3 {
                result[count] = [left as u8, right as u8, find(power)];
                count += 1;
            }
            right += 1;
        }
        left += 1;
    }
    assert!(count == 84);
    result
}
const fn derivatives() -> [[[u8; 3]; 10]; 3] {
    let mut result = [[[0; 3]; 10]; 3];
    let mut axis = 0;
    while axis < 3 {
        let mut out = 0;
        while out < 10 {
            let mut power = POWERS[out];
            power[axis] += 1;
            let source = find(power);
            assert!(source < COUNT as u8);
            result[axis][out] = [out as u8, source, power[axis]];
            out += 1;
        }
        axis += 1;
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn runtime_tables_match_every_const_entry_and_refuse_an_absent_degree() {
        assert_eq!(powers(), POWERS);
        assert_eq!(products(), PRODUCTS);
        assert_eq!(derivatives(), DERIVATIVES);
        assert_eq!(find([4, 0, 0]), COUNT as u8);
        for (index, power) in POWERS.into_iter().enumerate() {
            assert_eq!(find(power), index as u8);
        }
    }
}
