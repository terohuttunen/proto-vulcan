use crate::goal::Goal;
use crate::lterm::LTerm;
use crate::operator::conde;
use crate::relation::conso;
use crate::relation::rembero;
use crate::relation::resto;
use crate::user::UserState;
use std::rc::Rc;

/// A relation that will permute xl into yl.
pub fn permuteo<U: UserState>(xl: &Rc<LTerm>, yl: &Rc<LTerm>) -> Rc<dyn Goal<U>> {
    let xl = Rc::clone(xl);
    let yl = Rc::clone(yl);
    proto_vulcan!(
        closure {
            conde {
                [xl == [], yl == []],
                |x, xs, ys| {
                    conso(x, xs, xl),
                    resto(yl, ys),
                    permuteo(xs, ys),
                    rembero(x, yl, ys),
                }
            }
        }
    )
}
