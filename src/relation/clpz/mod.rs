pub mod ltez;
pub mod ltz;
pub mod plusz;
pub mod timesz;

#[cfg(test)]
mod integration_tests {
    use super::ltez::ltez;
    use super::ltz::ltz;
    use super::plusz::plusz;
    use crate::prelude::*;

    /// Test poso function using CLPZ inequalities
    #[test]
    fn test_poso_clpz_integration() {
        // Test that poso(x) works with CLPZ > 0 constraint
        let query = proto_vulcan_query!(|x| {
            // Simulating poso(x) as x > 0 using CLPZ
            ltz(0, x),  // 0 < x, equivalent to x > 0
            x == 5
        });

        let mut iter = query.run();
        assert_eq!(iter.next().unwrap().x, 5);
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_poso_clpz_integration_fail() {
        // Test that poso(x) fails for non-positive numbers
        let query = proto_vulcan_query!(|x| {
            ltz(0, x),  // x > 0
            x == 0      // Should fail since 0 is not > 0
        });

        let mut iter = query.run();
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_sub1o_clpz_integration() {
        // Test successor/predecessor relation using CLPZ arithmetic
        let query = proto_vulcan_query!(|n1| {
            |n| {
                // n == n1 + 1, simulating sub1o(n, n1)
                plusz(n1, 1, n),
                n == 5,
                n1 == 4
            }
        });

        let mut iter = query.run();
        assert_eq!(iter.next().unwrap().n1, 4);
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_arithmetic_bounds_checking() {
        // Test that CLPZ can enforce bounds like poso would
        let query = proto_vulcan_query!(|result| {
            |x, y| {
                // Order: ground values first, then arithmetic, then inequalities
                x == 3,
                y == 7,
                plusz(x, y, result),
                ltz(0, x),      // x > 0 (poso-like)
                ltz(0, y)       // y > 0 (poso-like)
            }
        });

        let mut iter = query.run();
        assert_eq!(iter.next().unwrap().result, 10);
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_range_constraints() {
        // Test constraining values to a range using inequalities
        let query = proto_vulcan_query!(|x| {
            // 1 <= x <= 10
            ltez(1, x),     // 1 <= x
            ltez(x, 10),    // x <= 10
            x == 5          // Should satisfy the range
        });

        let mut iter = query.run();
        assert_eq!(iter.next().unwrap().x, 5);
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_range_constraints_fail_lower() {
        // Test range constraint failure (too low)
        let query = proto_vulcan_query!(|x| {
            ltez(1, x),     // 1 <= x
            ltez(x, 10),    // x <= 10
            x == 0          // Should fail since 0 < 1
        });

        let mut iter = query.run();
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_range_constraints_fail_upper() {
        // Test range constraint failure (too high)
        let query = proto_vulcan_query!(|x| {
            ltez(1, x),     // 1 <= x
            ltez(x, 10),    // x <= 10
            x == 15         // Should fail since 15 > 10
        });

        let mut iter = query.run();
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_ordering_constraints() {
        // Test ordering multiple variables with inequalities
        let query = proto_vulcan_query!(|a| {
            |b, c| {
                // Enforce a < b < c
                ltz(a, b),
                ltz(b, c),
                a == 1,
                c == 5,
                b == 3      // Should satisfy 1 < 3 < 5
            }
        });

        let mut iter = query.run();
        assert_eq!(iter.next().unwrap().a, 1);
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_ordering_constraints_fail() {
        // Test ordering constraint failure
        let query = proto_vulcan_query!(|a| {
            |b, c| {
                ltz(a, b),   // a < b
                ltz(b, c),   // b < c
                a == 5,
                c == 1,      // Should fail since we can't have 5 < b < 1
                b == 3
            }
        });

        let mut iter = query.run();
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_mixed_arithmetic_and_ordering() {
        // Test combining arithmetic and ordering constraints
        let query = proto_vulcan_query!(|result| {
            |x, y, sum| {
                // Order: ground values first, then arithmetic, then inequalities
                x == 3,
                y == 4,              // sum = 7, satisfies 0 < 7 < 10
                plusz(x, y, sum),    // sum = x + y
                result == sum,
                ltz(0, x),           // x > 0
                ltz(0, y),           // y > 0
                ltz(sum, 10)         // sum < 10
            }
        });

        let mut iter = query.run();
        assert_eq!(iter.next().unwrap().result, 7);
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_constraint_propagation_ordering() {
        // Test that constraints propagate correctly through the system
        let query = proto_vulcan_query!(|middle| {
            |low, high| {
                ltz(low, middle),    // low < middle
                ltz(middle, high),   // middle < high
                low == 10,
                high == 20,
                middle == 15         // Should satisfy 10 < 15 < 20
            }
        });

        let mut iter = query.run();
        assert_eq!(iter.next().unwrap().middle, 15);
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_order_dependency() {
        // Test with arithmetic constraints before inequality constraints
        let query = proto_vulcan_query!(|result| {
            |x, y| {
                x == 3,
                y == 7,
                plusz(x, y, result),    // arithmetic first
                ltz(0, x),              // then inequalities
                ltz(0, y)
            }
        });

        let mut iter = query.run();
        let result = iter.next();
        println!("Order test result: {:?}", result);
        assert!(result.is_some());
        assert_eq!(result.unwrap().result, 10);
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_simple_ltz_debug() {
        // Simplified version of the failing test
        let query = proto_vulcan_query!(|x| {
            ltz(0, x),  // x > 0
            x == 3
        });

        let mut iter = query.run();
        let result = iter.next();
        println!("Debug result: {:?}", result);
        assert!(result.is_some());
        assert_eq!(result.unwrap().x, 3);
        assert!(iter.next().is_none());
    }
}
