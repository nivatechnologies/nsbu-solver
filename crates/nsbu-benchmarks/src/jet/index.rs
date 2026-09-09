//! Compile-time dense degree-four indices and the 495 valid convolution products.
pub(super) const COUNT: usize = 70;
pub(super) const POWERS: [[u8; 4]; COUNT] = powers();
pub(super) const PRODUCTS: [[u8; 3]; 495] = products();

const fn powers() -> [[u8; 4]; COUNT] {
    let mut result = [[0; 4]; COUNT];
    let mut index = 0;
    let mut degree = 0;
    while degree <= 4 {
        let mut a = 0;
        while a <= degree {
            let mut b = 0;
            while b <= degree - a {
                let mut c = 0;
                while c <= degree - a - b {
                    result[index] = [a, b, c, degree - a - b - c];
                    index += 1;
                    c += 1;
                }
                b += 1;
            }
            a += 1;
        }
        degree += 1;
    }
    result
}

const fn find(power: [u8; 4]) -> u8 {
    let mut index = 0;
    while index < COUNT {
        let value = POWERS[index];
        if value[0] == power[0]
            && value[1] == power[1]
            && value[2] == power[2]
            && value[3] == power[3]
        {
            return index as u8;
        }
        index += 1;
    }
    COUNT as u8
}

const fn products() -> [[u8; 3]; 495] {
    let mut result = [[0; 3]; 495];
    let mut count = 0;
    let mut left = 0;
    while left < COUNT {
        let mut right = 0;
        while right < COUNT {
            let a = POWERS[left];
            let b = POWERS[right];
            let power = [a[0] + b[0], a[1] + b[1], a[2] + b[2], a[3] + b[3]];
            if power[0] + power[1] + power[2] + power[3] <= 4 {
                result[count] = [left as u8, right as u8, find(power)];
                count += 1;
            }
            right += 1;
        }
        left += 1;
    }
    assert!(count == 495);
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_tables_cover_each_admissible_multi_index_and_product() {
        let runtime = powers();
        assert_eq!(runtime, POWERS);
        let mut unique = std::collections::BTreeSet::new();
        for power in runtime {
            assert!(power.iter().sum::<u8>() <= 4);
            assert!(unique.insert(power));
            assert_eq!(runtime[find(power) as usize], power);
        }
        assert_eq!(unique.len(), 70);
        assert_eq!(find([5, 0, 0, 0]), 70);
        let runtime_products = products();
        assert_eq!(runtime_products, PRODUCTS);
        let mut pairs = std::collections::BTreeSet::new();
        for [a, b, out] in runtime_products {
            let combined: [u8; 4] =
                std::array::from_fn(|i| runtime[a as usize][i] + runtime[b as usize][i]);
            assert_eq!(combined, runtime[out as usize]);
            assert!(pairs.insert((a, b)));
        }
        assert_eq!(pairs.len(), 495);
    }
}
