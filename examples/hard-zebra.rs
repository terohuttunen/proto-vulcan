extern crate proto_vulcan;
use itertools::izip;
use proto_vulcan::lterm::LTerm;
use proto_vulcan::relation::membero;
use proto_vulcan::relation::permuteo;
use proto_vulcan::*;
use std::time::Instant;

fn lefto(x: LTerm, y: LTerm, l: LTerm) -> Goal {
    proto_vulcan_closure!(
        match l {
            [head | rest] => {
                head == x,
                membero(y, rest),
            },
            [_ | rest] => lefto(x, y, rest),
        }
    )
}

// Of Landon and Jason, one has the 7:30pm reservation and the other loves mozzarella.
fn rule1(answers: LTerm) -> Goal {
    proto_vulcan!(
        |c1, r1, c2, r2| {
            membero(["landon", _, c1, r1], answers),
            membero(["jason", _, c2, r2], answers),
            conde {
                [r1 == "7:30pm", c2 == "mozzarella"],
                [r2 == "7:30pm", c1 == "mozzarella"],
            }
        }
    )
}

// The blue-cheese enthusiast subscribed to Fortune.
fn rule2(answers: LTerm) -> Goal {
    proto_vulcan!(membero([_, "fortune", "blue-cheese", _], answers))
}

// The muenster enthusiast didn't subscribe to Vogue.
fn rule3(answers: LTerm) -> Goal {
    proto_vulcan!(
        |s1, s2| {
            [_, "vogue", _, _] == s1,
            [_, _, "muenster", _] == s2,
            membero(s1, answers),
            membero(s2, answers),
            s1 != s2,
        }
    )
}

// The 5 people were the Fortune subscriber, Landon, the person with a
// reservation at 5:00pm, the mascarpone enthusiast, and the Vogue
// subscriber.
fn rule4(answers: LTerm) -> Goal {
    proto_vulcan!(
        permuteo(
            [[_, "fortune", _, _], ["landon", _, _, _], [_, _, _, "5:00pm"], [_, _, "mascarpone", _], [_, "vogue", _, _]],
            answers,
        )
    )
}

// The person with a reservation at 5:00pm didn't subscribe to Time.
fn rule5(answers: LTerm) -> Goal {
    proto_vulcan!(
        |s1, s2| {
            [_, _, _, "5:00pm"] == s1,
            [_, "time", _, _] == s2,
            membero(s1, answers),
            membero(s2, answers),
            s1 != s2,
        }
    )
}

// The Cosmopolitan subscriber has an earlier reservation than the mascarpone enthusiast.
fn rule6(answers: LTerm) -> Goal {
    proto_vulcan!(
        |r1, r2| {
            membero([_, "cosmopolitan", _, r1], answers),
            membero([_, _, "mascarpone", r2], answers),
            lefto(r1, r2, ["5:00pm", "6:00pm", "7:00pm", "7:30pm", "8:30pm"])
        }
    )
}

// Bailey has a later reservation than the blue-cheese enthusiast.
fn rule7(answers: LTerm) -> Goal {
    proto_vulcan!(
        |r1, r2| {
            membero([_, _, "blue-cheese", r1], answers),
            membero(["bailey", _, _, r2], answers),
            lefto(r1, r2, ["5:00pm", "6:00pm", "7:00pm", "7:30pm", "8:30pm"])
        }
    )
}

// Either the person with a reservation at 7:00pm or the person with a
// reservation at 7:30pm subscribed to Fortune.
fn rule8(answers: LTerm) -> Goal {
    proto_vulcan!(
        |r| {
            membero([_, "fortune", _, r], answers),
            conde {
                r == "7:00pm",
                r == "7:30pm",
            }
        }
    )
}

// Landon has a later reservation than the Time subscriber.
fn rule9(answers: LTerm) -> Goal {
    proto_vulcan!(
        |r1, r2| {
            membero([_, "time", _, r1], answers),
            membero(["landon", _, _, r2], answers),
            lefto(r1, r2, ["5:00pm", "6:00pm", "7:00pm", "7:30pm", "8:30pm"])
        }
    )
}

// The Fortune subscriber is not Jamari.
fn rule10(answers: LTerm) -> Goal {
    proto_vulcan!(
        |s1, s2| {
            [_, "fortune", _, _] == s1,
            ["jamari", _, _, _] == s2,
            membero(s1, answers),
            membero(s2, answers),
            s1 != s2,
        }
    )
}

// The person with a reservation at 5:00pm loves mozzarella.
fn rule11(answers: LTerm) -> Goal {
    proto_vulcan!(membero([_, _, "mozzarella", "5:00pm"], answers))
}

fn hard_zebrao(answers: LTerm) -> Goal {
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
        permuteo(
            magazines,
            ["fortune", "time", "cosmopolitan", "us-weekly", "vogue"]
        ),
        permuteo(
            cheeses,
            [
                "asiago",
                "blue-cheese",
                "mascarpone",
                "mozzarella",
                "muenster"
            ]
        ),
        permuteo(
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
