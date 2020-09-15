extern crate proto_vulcan;
use proto_vulcan::relation::conso;
use proto_vulcan::relation::firsto;
use proto_vulcan::relation::membero;
use proto_vulcan::*;
use std::rc::Rc;
use std::time::Instant;

/* The puzzle:
        1. There are five houses.
        2. The Englishman lives in the red house.
        3. The Spaniard owns the dog.
        4. Coffee is drunk in the green house.
        5. The Ukrainian drinks tea.
        6. The green house is immediately to the right of the ivory house.
        7. The Old Gold smoker owns snails.
        8. Kools are smoked in the yellow house.
        9. Milk is drunk in the middle house.
        10. The Norwegian lives in the first house.
        11. The man who smokes Chesterfields lives in the house next to the man with the fox.
        12. Kools are smoked in the house next to the house where the horse is kept.
        13. The Lucky Strike smoker drinks orange juice.
        14. The Japanese smokes Parliaments.
        15. The Norwegian lives next to the blue house.

    Now, who drinks water? Who owns the zebra?

    In the interest of clarity, it must be added that each of the five houses is painted a
    different color, and their inhabitants are of different national extractions, own
    different pets, drink different beverages and smoke different brands of American cigarets
    [sic]. One other thing: in statement 6, right means your right.

    — Life International, December 17, 1962

*/

fn righto(x: &Rc<LTerm>, y: &Rc<LTerm>, l: &Rc<LTerm>) -> Rc<dyn Goal> {
    let x = Rc::clone(x);
    let y = Rc::clone(y);
    proto_vulcan!(|first, rest| {
        conso(first, rest, l),
        conde {
            |second| {
                firsto(rest, second),
                first == y,
                second == x
            },
            closure { righto(x, y, rest) }
        }
    })
}

fn nexto(x: &Rc<LTerm>, y: &Rc<LTerm>, l: &Rc<LTerm>) -> Rc<dyn Goal> {
    proto_vulcan!(conde {
        righto(x, y, l),
        righto(y, x, l)
    })
}

fn medium_zebrao(houses: &Rc<LTerm>) -> Rc<dyn Goal> {
    proto_vulcan!([
        [_, _, [_, _, "milk", _, _], _, _] == houses,
        firsto(houses, ["norwegian", _, _, _, _]),
        nexto(["norwegian", _, _, _, _], [_, _, _, _, "blue"], houses),
        righto([_, _, _, _, "ivory"], [_, _, _, _, "green"], houses),
        membero(["englishman", _, _, _, "red"], houses),
        membero([_, "kools", _, _, "yellow"], houses),
        membero(["spaniard", _, _, "dog", _], houses),
        membero([_, _, "coffee", _, "green"], houses),
        membero(["ukrainian", _, "tea", _, _], houses),
        membero([_, "lucky-strikes", "oj", _, _], houses),
        membero(["japanese", "parliaments", _, _, _], houses),
        membero([_, "oldgolds", _, "snails", _], houses),
        nexto([_, _, _, "horse", _], [_, "kools", _, _, _], houses),
        nexto([_, _, _, "fox", _], [_, "chesterfields", _, _, _], houses)
    ])
}

fn main() {
    let zebra = proto_vulcan_query!(|houses| { medium_zebrao(houses) });

    let start = Instant::now();
    let mut iter = zebra.run();
    let result = iter.next().unwrap();
    let duration = start.elapsed();

    println!("{}", result);
    println!("Time elapsed: {:?}", duration);
}
