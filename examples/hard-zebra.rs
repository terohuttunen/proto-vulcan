extern crate proto_vulcan;
use itertools::izip;
use proto_vulcan::lterm::LTerm;
use proto_vulcan::prelude::*;
use proto_vulcan::relation::member;
use proto_vulcan::relation::permute;
use std::time::Instant;

fn lefto<U: User, E: Engine<U>>(x: LTerm<U, E>, y: LTerm<U, E>, l: LTerm<U, E>) -> Goal<U, E> {
    proto_vulcan_closure!(
        match l {
            [head | rest] => {
                head == x,
                member(y, rest),
            },
            [_ | rest] => lefto(x, y, rest),
        }
    )
}

// Of Landon and Jason, one has the 7:30pm reservation and the other loves mozzarella.
fn rule1<U: User, E: Engine<U>>(answers: LTerm<U, E>) -> Goal<U, E> {
    proto_vulcan!(
        |c1, r1, c2, r2| {
            member(["landon", _, c1, r1], answers),
            member(["jason", _, c2, r2], answers),
            conde {
                [r1 == "7:30pm", c2 == "mozzarella"],
                [r2 == "7:30pm", c1 == "mozzarella"],
            }
        }
    )
}

// The blue-cheese enthusiast subscribed to Fortune.
fn rule2<U: User, E: Engine<U>>(answers: LTerm<U, E>) -> Goal<U, E> {
    proto_vulcan!(member([_, "fortune", "blue-cheese", _], answers))
}

// The muenster enthusiast didn't subscribe to Vogue.
fn rule3<U: User, E: Engine<U>>(answers: LTerm<U, E>) -> Goal<U, E> {
    proto_vulcan!(
        |s1, s2| {
            [_, "vogue", _, _] == s1,
            [_, _, "muenster", _] == s2,
            member(s1, answers),
            member(s2, answers),
            s1 != s2,
        }
    )
}

// The 5 people were the Fortune subscriber, Landon, the person with a
// reservation at 5:00pm, the mascarpone enthusiast, and the Vogue
// subscriber.
fn rule4<U: User, E: Engine<U>>(answers: LTerm<U, E>) -> Goal<U, E> {
    proto_vulcan!(permute(
        [
            [_, "fortune", _, _],
            ["landon", _, _, _],
            [_, _, _, "5:00pm"],
            [_, _, "mascarpone", _],
            [_, "vogue", _, _]
        ],
        answers,
    ))
}

// The person with a reservation at 5:00pm didn't subscribe to Time.
fn rule5<U: User, E: Engine<U>>(answers: LTerm<U, E>) -> Goal<U, E> {
    proto_vulcan!(
        |s1, s2| {
            [_, _, _, "5:00pm"] == s1,
            [_, "time", _, _] == s2,
            member(s1, answers),
            member(s2, answers),
            s1 != s2,
        }
    )
}

// The Cosmopolitan subscriber has an earlier reservation than the mascarpone enthusiast.
fn rule6<U: User, E: Engine<U>>(answers: LTerm<U, E>) -> Goal<U, E> {
    proto_vulcan!(
        |r1, r2| {
            member([_, "cosmopolitan", _, r1], answers),
            member([_, _, "mascarpone", r2], answers),
            lefto(r1, r2, ["5:00pm", "6:00pm", "7:00pm", "7:30pm", "8:30pm"])
        }
    )
}

// Bailey has a later reservation than the blue-cheese enthusiast.
fn rule7<U: User, E: Engine<U>>(answers: LTerm<U, E>) -> Goal<U, E> {
    proto_vulcan!(
        |r1, r2| {
            member([_, _, "blue-cheese", r1], answers),
            member(["bailey", _, _, r2], answers),
            lefto(r1, r2, ["5:00pm", "6:00pm", "7:00pm", "7:30pm", "8:30pm"])
        }
    )
}

// Either the person with a reservation at 7:00pm or the person with a
// reservation at 7:30pm subscribed to Fortune.
fn rule8<U: User, E: Engine<U>>(answers: LTerm<U, E>) -> Goal<U, E> {
    proto_vulcan!(
        |r| {
            member([_, "fortune", _, r], answers),
            conde {
                r == "7:00pm",
                r == "7:30pm",
            }
        }
    )
}

// Landon has a later reservation than the Time subscriber.
fn rule9<U: User, E: Engine<U>>(answers: LTerm<U, E>) -> Goal<U, E> {
    proto_vulcan!(
        |r1, r2| {
            member([_, "time", _, r1], answers),
            member(["landon", _, _, r2], answers),
            lefto(r1, r2, ["5:00pm", "6:00pm", "7:00pm", "7:30pm", "8:30pm"])
        }
    )
}

// The Fortune subscriber is not Jamari.
fn rule10<U: User, E: Engine<U>>(answers: LTerm<U, E>) -> Goal<U, E> {
    proto_vulcan!(
        |s1, s2| {
            [_, "fortune", _, _] == s1,
            ["jamari", _, _, _] == s2,
            member(s1, answers),
            member(s2, answers),
            s1 != s2,
        }
    )
}

// The person with a reservation at 5:00pm loves mozzarella.
fn rule11<U: User, E: Engine<U>>(answers: LTerm<U, E>) -> Goal<U, E> {
    proto_vulcan!(member([_, _, "mozzarella", "5:00pm"], answers))
}

fn hard_zebrao<U: User, E: Engine<U>>(answers: LTerm<U, E>) -> Goal<U, E> {
    let people = lterm!([_, _, _, _, _]);
    let magazines = lterm!([_, _, _, _, _]);
    let cheeses = lterm!([_, _, _, _, _]);
    let reservations = lterm!([_, _, _, _, _]);
    let mut ans = lterm!([]);
    for (p, m, c, r) in izip!(
        people.iter(),
        magazines.iter(),
        cheeses.iter(),
        reservations.iter()
    ) {
        ans.extend(Some(lterm!([p, m, c, r])));
    }
    proto_vulcan!([
        answers == ans,
        people == ["amaya", "bailey", "jamari", "jason", "landon"],
        rule1(answers),
        rule2(answers),
        rule3(answers),
        rule4(answers),
        rule5(answers),
        rule6(answers),
        rule7(answers),
        rule8(answers),
        rule9(answers),
        rule10(answers),
        rule11(answers),
        permute(
            magazines,
            ["fortune", "time", "cosmopolitan", "us-weekly", "vogue"]
        ),
        permute(
            cheeses,
            [
                "asiago",
                "blue-cheese",
                "mascarpone",
                "mozzarella",
                "muenster"
            ]
        ),
        permute(
            reservations,
            ["5:00pm", "6:00pm", "7:00pm", "7:30pm", "8:30pm"]
        ),
    ])
}

fn main() {
    let zebra = proto_vulcan_query!(|answers| { hard_zebrao(answers) });

    let start = Instant::now();
    for result in zebra.run() {
        println!("{}", result);
    }
    let duration = start.elapsed();
    println!("Time elapsed: {:?}", duration);
}
