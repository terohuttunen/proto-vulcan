
use crate::goal::Goal;


/// A relation that succeeds an unbounded number of times.
pub fn always() -> Goal
{
    proto_vulcan!(loop {
        true
    })
}

#[cfg(test)]
mod tests {
    use super::always;
    use crate::operator::conde::conde;
    use crate::prelude::*;

    #[test]
    fn test_always_1() {
        let query = proto_vulcan_query!(|x| {
            conde {
                true == x,
                false == x,
            },
            always(),
            false == x,
        });
        let mut iter = query.run();
        assert_eq!(iter.next().unwrap().x, false);
        assert_eq!(iter.next().unwrap().x, false);
        assert_eq!(iter.next().unwrap().x, false);
        assert_eq!(iter.next().unwrap().x, false);
        assert_eq!(iter.next().unwrap().x, false);
    }
}
