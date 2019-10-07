use std::str::FromStr;

#[derive(Debug, Clone)]
pub enum ResponderBehaviour {
    AlwaysFirst,
    AlwaysLast,
    Random,
    SequentialClamping,
    SequentialOnce,
    SequentialWrapping,
}

impl ResponderBehaviour {
    pub fn choose_index(&self, last: Option<usize>, length: usize) -> Option<usize> {
        if length < 1 {
            return None;
        }

        match self {
            Self::AlwaysFirst => Some(0),
            Self::AlwaysLast => Some(length - 1),
            Self::Random => {
                use rand::prelude::*;
                let mut rng = thread_rng();
                Some(rng.gen_range(0, length))
            }
            Self::SequentialClamping | Self::SequentialOnce | Self::SequentialWrapping => {
                match last {
                    Some(mut last) => {
                        last += 1;
                        if last >= length {
                            match self {
                                Self::SequentialClamping => Some(length - 1),
                                Self::SequentialOnce => None,
                                Self::SequentialWrapping => Some(0),
                                _ => unreachable!(),
                            }
                        } else {
                            Some(last)
                        }
                    }
                    None => Some(0),
                }
            }
        }
    }

    pub fn variants() -> &'static [&'static str] {
        &[
            "always-first",
            "always-last",
            "random",
            "sequential-clamping",
            "sequential-once",
            "sequential-wrapping",
        ]
    }
}

impl FromStr for ResponderBehaviour {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "always-first" => Ok(Self::AlwaysFirst),
            "always-last" => Ok(Self::AlwaysLast),
            "random" => Ok(Self::Random),
            "sequential-clamping" => Ok(Self::SequentialClamping),
            "sequential-once" => Ok(Self::SequentialOnce),
            "sequential-wrapping" => Ok(Self::SequentialWrapping),
            _ => Err("Unrecognized behaviour option"),
        }
    }
}

#[cfg(test)]
mod tests {
    use test_case::test_case;

    use super::ResponderBehaviour::{self, *};

    // Zero-length
    #[test_case(SequentialWrapping, Some(0), 0, None)]
    #[test_case(SequentialClamping, Some(0), 0, None)]
    #[test_case(SequentialOnce, Some(0), 0, None)]
    #[test_case(Random, Some(0), 0, None)]
    #[test_case(AlwaysFirst, Some(0), 0, None)]
    #[test_case(AlwaysLast, Some(0), 0, None)]
    #[test_case(SequentialWrapping, Some(1), 0, None)]
    #[test_case(SequentialClamping, Some(1), 0, None)]
    #[test_case(SequentialOnce, Some(1), 0, None)]
    #[test_case(Random, Some(1), 0, None)]
    #[test_case(AlwaysFirst, Some(1), 0, None)]
    #[test_case(AlwaysLast, Some(1), 0, None)]
    // AlwaysFirst
    #[test_case(AlwaysFirst, None, 1, Some(0))]
    #[test_case(AlwaysFirst, Some(0), 1, Some(0))]
    #[test_case(AlwaysFirst, Some(1), 1, Some(0))]
    #[test_case(AlwaysFirst, Some(2), 1, Some(0))]
    #[test_case(AlwaysFirst, Some(3), 1, Some(0))]
    // AlwaysLast
    #[test_case(AlwaysLast, None, 1, Some(0))]
    #[test_case(AlwaysLast, Some(3), 1, Some(0))]
    #[test_case(AlwaysLast, Some(3), 2, Some(1))]
    #[test_case(AlwaysLast, Some(3), 3, Some(2))]
    #[test_case(AlwaysLast, Some(3), 4, Some(3))]
    #[test_case(AlwaysLast, Some(3), 5, Some(4))]
    // SequentialWrapping
    #[test_case(SequentialWrapping, None, 4, Some(0))]
    #[test_case(SequentialWrapping, Some(0), 4, Some(1))]
    #[test_case(SequentialWrapping, Some(1), 4, Some(2))]
    #[test_case(SequentialWrapping, Some(2), 4, Some(3))]
    #[test_case(SequentialWrapping, Some(3), 4, Some(0))]
    #[test_case(SequentialWrapping, Some(5), 4, Some(0))]
    #[test_case(SequentialWrapping, Some(6), 4, Some(0))]
    #[test_case(SequentialWrapping, Some(7), 4, Some(0))]
    #[test_case(SequentialWrapping, Some(8), 4, Some(0))]
    #[test_case(SequentialWrapping, Some(9), 4, Some(0))]
    // SequentialClamping
    #[test_case(SequentialClamping, None, 4, Some(0))]
    #[test_case(SequentialClamping, Some(0), 4, Some(1))]
    #[test_case(SequentialClamping, Some(1), 4, Some(2))]
    #[test_case(SequentialClamping, Some(2), 4, Some(3))]
    #[test_case(SequentialClamping, Some(3), 4, Some(3))]
    #[test_case(SequentialClamping, Some(4), 4, Some(3))]
    #[test_case(SequentialClamping, Some(5), 4, Some(3))]
    #[test_case(SequentialClamping, Some(6), 4, Some(3))]
    #[test_case(SequentialClamping, Some(7), 4, Some(3))]
    #[test_case(SequentialClamping, Some(8), 4, Some(3))]
    #[test_case(SequentialClamping, Some(9), 4, Some(3))]
    // SequentialOnce
    #[test_case(SequentialOnce, None, 4, Some(0))]
    #[test_case(SequentialOnce, Some(0), 4, Some(1))]
    #[test_case(SequentialOnce, Some(1), 4, Some(2))]
    #[test_case(SequentialOnce, Some(2), 4, Some(3))]
    #[test_case(SequentialOnce, Some(3), 4, None)]
    #[test_case(SequentialOnce, Some(4), 4, None)]
    #[test_case(SequentialOnce, Some(5), 4, None)]
    #[test_case(SequentialOnce, Some(6), 4, None)]
    #[test_case(SequentialOnce, Some(7), 4, None)]
    #[test_case(SequentialOnce, Some(8), 4, None)]
    #[test_case(SequentialOnce, Some(9), 4, None)]
    fn behaviour_tests(
        variant: ResponderBehaviour,
        last: Option<usize>,
        length: usize,
        expected: Option<usize>,
    ) {
        assert_eq!(variant.choose_index(last, length), expected);
    }

    // #[test]
    // fn it_works() {
    //     assert_eq!(2 + 2, 4);
    // }
}
