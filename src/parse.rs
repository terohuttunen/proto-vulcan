#[doc(hidden)]
#[macro_export]
macro_rules! comma_separated {
    {
        $caller:tt
        parser = [{ $parser:ident }]
        input = [{ $($input:tt)* }]
    } => {
        $crate::tt_call::tt_call! {
            macro = [{ $parser }]
            input = [{ $($input)* }]
            ~~> private_comma_separated! {
                $caller
                parser = [{ $parser }]
                elements = [{ }]
            }
        }
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! private_comma_separated {
    // Finished without trailing comma.
    {
        $caller:tt
        parser = [{ $parser:ident }]
        elements = [{ $($elements:tt)* }]
        $name:ident = [{ $($current:tt)* }]
        rest = [{ }]
    } => {
        $crate::tt_call::tt_return! {
            $caller
            $($elements)*
            element = [{ $($current)* }]
        }
    };

    // Finished after ignoring trailing comma.
    {
        $caller:tt
        parser = [{ $parser:ident }]
        elements = [{ $($elements:tt)* }]
        $name:ident = [{ $($current:tt)* }]
        rest = [{ , }]
    } => {
        $crate::tt_call::tt_return! {
            $caller
            $($elements)*
            element = [{ $($current)* }]
        }
    };

    // Parse next element after comma.
    {
        $caller:tt
        parser = [{ $parser:ident }]
        elements = [{ $($elements:tt)* }]
        $name:ident = [{ $($current:tt)* }]
        rest = [{ , $($rest:tt)+ }]
    } => {
        $crate::tt_call::tt_call! {
            macro = [{ $parser }]
            input = [{ $($rest)* }]
            ~~> private_comma_separated! {
                $caller
                parser = [{ $parser }]
                elements = [{
                    $($elements)*
                    element = [{ $($current)* }]
                }]
            }
        }
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! private_parse_parameter {
    {
        $caller:tt
        input = [{ # $tt:expr }]
    } => {
        $crate::tt_call::tt_return! {
            $caller
            output = [{ $tt }]
            rest = [{ }]
        }
    };

    {
        $caller:tt
        input = [{ # $tt:expr , $( $rest:tt )* }]
    } => {
        $crate::tt_call::tt_return! {
            $caller
            output = [{ $tt }]
            rest = [{ , $( $rest )* }]
        }
    };

    {
        $caller:tt
        input = [{ $tt:ident $( $rest:tt )* }]
    } => {
        $crate::tt_call::tt_return! {
            $caller
            output = [{ &lterm!($tt) }]
            rest = [{ $( $rest )* }]
        }
    };

    {
        $caller:tt
        input = [{ $tt:tt $( $rest:tt )* }]
    } => {
        $crate::tt_call::tt_return! {
            $caller
            output = [{ &lterm!($tt) }]
            rest = [{ $( $rest )* }]
        }
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! private_parse_relation {
    // empty parameter list
    {
        $caller:tt
        input = [{ $( $relation:ident )::+ ( ) $( $rest:tt )* }]
    } => {
        $crate::tt_call::tt_return! {
            $caller
            output = [{ $( $relation )::+ ( ) }]
            rest = [{ $( $rest )* }]
        }
    };

    // return after parsing all parameters
    {
        $caller:tt
        relation = [{ $( $relation:ident )::+ }]
        rest = [{ $( $rest:tt )* }]
        $(
            element = [{ $( $element:tt )* }]
        )*
    } => {
        $crate::tt_call::tt_return! {
            $caller
            output = [{ $( $relation )::+ ( $( $( $element )* ),* ) }]
            rest = [{ $( $rest )* }]
        }
    };

    // non-empty parameter list
    {
        $caller:tt
        input = [{ $( $relation:ident )::+ ( $( $param:tt )* ) $( $rest:tt )* }]
    } => {
        $crate::tt_call::tt_call! {
            macro = [{ comma_separated }]
            parser = [{ private_parse_parameter }]
            input = [{ $( $param )* }]
            ~~> private_parse_relation! {
                $caller
                relation = [{ $( $relation )::+ }]
                rest = [{ $( $rest )* }]
            }
        }
    };
}

// Conjunction of list of clauses: [clause1 AND clause2 AND ... ]
#[doc(hidden)]
#[macro_export]
macro_rules! private_parse_conjunction_list {
    // return after parsing all clauses
    {
        $caller:tt
        rest = [{ $( $rest:tt )* }]
        $(
            element = [{ $( $element:tt )* }]
        )*
    } => {
        $crate::tt_call::tt_return! {
            $caller
            output = [{ $crate::operator::all::All::from_array( &[ $( $( $element )* ),* ] ) }]
            rest = [{ $( $rest )* }]
        }
    };

    {
        $caller:tt
        input = [{ [ $( $conjunction_list:tt )+ ] $( $rest:tt )* }]
    } => {
        $crate::tt_call::tt_call! {
            macro = [{ comma_separated }]
            parser = [{ private_parse_clause }]
            input = [{ $( $conjunction_list )* }]
            ~~> private_parse_conjunction_list! {
                $caller
                rest = [{ $( $rest )* }]
            }
        }
    };
}

// Conjunction of list of clauses: [clause1 AND clause2 AND ... ]
#[doc(hidden)]
#[macro_export]
macro_rules! private_parse_conjunction_list_in_operator {
    // return after parsing all clauses
    {
        $caller:tt
        rest = [{ $( $rest:tt )* }]
        $(
            element = [{ $( $element:tt )* }]
        )*
    } => {
        $crate::tt_call::tt_return! {
            $caller
            output = [{ &[ $( $( $element )* ),* ] }]
            rest = [{ $( $rest )* }]
        }
    };

    {
        $caller:tt
        input = [{ [ $( $conjunction_list:tt )+ ] $( $rest:tt )* }]
    } => {
        $crate::tt_call::tt_call! {
            macro = [{ comma_separated }]
            parser = [{ private_parse_clause }]
            input = [{ $( $conjunction_list )* }]
            ~~> private_parse_conjunction_list_in_operator! {
                $caller
                rest = [{ $( $rest )* }]
            }
        }
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! private_parse_operator {
    // return after parsing all body clauses for "loop" operator
    {
        $caller:tt
        operator = [{ loop }]
        rest = [{ $( $rest:tt )* }]
        $(
            element = [{ $( $element:tt )* }]
        )*
    } => {
        $crate::tt_call::tt_return! {
            $caller
            output = [{ $crate::operator::anyo::anyo ( &[ $( $( $element )* ),* ] ) }]
            rest = [{ $( $rest )* }]
        }
    };

    // return after parsing all body clauses
    {
        $caller:tt
        operator = [{ $( $operator:ident )::+ }]
        rest = [{ $( $rest:tt )* }]
        $(
            element = [{ $( $element:tt )* }]
        )*
    } => {
        $crate::tt_call::tt_return! {
            $caller
            output = [{ $( $operator )::+ ( &[ $( $( $element )* ),* ] )}]
            rest = [{ $( $rest )* }]
        }
    };

    {
        $caller:tt
        input = [{ $( $operator:ident )::+ { $( $body:tt )+ } $( $rest:tt )* }]
    } => {
        $crate::tt_call::tt_call! {
            macro = [{ comma_separated  }]
            parser = [{ private_parse_clause_in_operator }]
            input = [{ $( $body )* }]
            ~~> private_parse_operator! {
                $caller
                operator = [{ $( $operator )::+ }]
                rest = [{ $( $rest )* }]
            }
        }
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! private_parse_closure {
    // return after parsing all body clauses
    {
        $caller:tt
        rest = [{ $( $rest:tt )* }]
        $(
            element = [{ $( $element:tt )* }]
        )*
    } => {
        $crate::tt_call::tt_return! {
            $caller
            output = [{ $crate::operator::closure::Closure::new(Box::new(move || $crate::operator::all::All::from_array( &[ $( $( $element )* ),* ] ) ) ) }]
            rest = [{ $( $rest )* }]
        }
    };

    {
        $caller:tt
        input = [{ closure { $( $body:tt )+ } $( $rest:tt )* }]
    } => {
        $crate::tt_call::tt_call! {
            macro = [{ comma_separated  }]
            parser = [{ private_parse_clause }]
            input = [{ $( $body )* }]
            ~~> private_parse_closure! {
                $caller
                rest = [{ $( $rest )* }]
            }
        }
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! private_parse_project {
    // return after parsing all body clauses
    {
        $caller:tt
        variables = [{ $( $m:ident ),+ }]
        rest = [{ $( $rest:tt )* }]
        $(
            element = [{ $( $element:tt )* }]
        )*
    } => {
        $crate::tt_call::tt_return! {
            $caller
            output = [{ { $( let $m = $crate::lterm::LTerm::projection($m); )+ $crate::operator::project::Project::new(vec![ $( ::std::rc::Rc::clone(&$m) ),+ ], $crate::operator::all::All::from_array( &[ $( $( $element )* ),* ] ) ) } }]
            rest = [{ $( $rest )* }]
        }
    };

    {
        $caller:tt
        input = [{ project | $( $m:ident ),+ | { $( $body:tt )+ } $( $rest:tt )* }]
    } => {
        $crate::tt_call::tt_call! {
            macro = [{ comma_separated  }]
            parser = [{ private_parse_clause }]
            input = [{ $( $body )* }]
            ~~> private_parse_project! {
                $caller
                variables = [{ $( $m ),+ }]
                rest = [{ $( $rest )* }]
            }
        }
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! private_parse_fresh {
    // return after parsing all body clauses
    {
        $caller:tt
        variables = [{ $( $m:ident ),+ }]
        rest = [{ $( $rest:tt )* }]
        $(
            element = [{ $( $element:tt )* }]
        )*
    } => {
        $crate::tt_call::tt_return! {
            $caller
            output = [{ { $( let $m = $crate::lterm::LTerm::var(stringify!($m)); )+ $crate::operator::fresh::Fresh::new(vec![$( ::std::rc::Rc::clone(&$m) ),+ ], $crate::operator::all::All::from_array( &[ $( $( $element )* ),* ] ) ) } }]
            rest = [{ $( $rest )* }]
        }
    };

    {
        $caller:tt
        input = [{ | $( $m:ident ),+ | { $( $body:tt )+ } $( $rest:tt )* }]
    } => {
        $crate::tt_call::tt_call! {
            macro = [{ comma_separated  }]
            parser = [{ private_parse_clause }]
            input = [{ $( $body )* }]
            ~~> private_parse_fresh! {
                $caller
                variables = [{ $( $m ),+ }]
                rest = [{ $( $rest )* }]
            }
        }
    };
}

// When other than [] clauses are within an operator, they must be
// wrapped into &[], in order to always provide &[&[goal1, goal2, ..]] to
// operators as input.
#[doc(hidden)]
#[macro_export]
macro_rules! private_parse_clause_in_operator {
    // return from private_parse_clause_in_operator
    {
        $caller:tt
        output = [{ $( $output:tt )* }]
        rest = [{ $( $rest:tt )* }]
    } => {
        $crate::tt_call::tt_return! {
            $caller
            output = [{ &[ $( $output )* ] }]
            rest = [{ $( $rest )* }]
        }
    };

    // [ ] is passed to private_parse_clause and returned there as well.
    {
        $caller:tt
        input = [{ [ $( $conjunction_list:tt )+ ] $( $rest:tt )* }]
    } => {
        $crate::tt_call::tt_call! {
            macro = [{ private_parse_conjunction_list_in_operator }]
            input = [{ [ $( $conjunction_list )+ ] $( $rest )* }]
            ~~> private_parse_clause! {
                $caller
            }
        }
    };

    // Other clauses are returned to this macro
    {
        $caller:tt
        input = [{ $( $input:tt )* }]
    } => {
        $crate::tt_call::tt_call! {
            macro = [{ private_parse_clause }]
            input = [{ $( $input )* }]
            ~~> private_parse_clause_in_operator! {
                $caller
            }
        }
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! private_parse_clause {
    // return from private_parse_clause
    {
        $caller:tt
        output = [{ $( $output:tt )* }]
        rest = [{ $( $rest:tt )* }]
    } => {
        $crate::tt_call::tt_return! {
            $caller
            output = [{ $( $output )* }]
            rest = [{ $( $rest )* }]
        }
    };

    {
        $caller:tt
        input = [{ project | $( $m:ident ),+ | { $( $body:tt )+ } $( $rest:tt )* }]
    } => {

        $crate::tt_call::tt_call! {
            macro = [{ private_parse_project }]
            input = [{ project | $( $m ),+ | { $( $body )+ } $( $rest )* }]
            ~~> private_parse_clause! {
                $caller
            }
        }
    };

    {
        $caller:tt
        input = [{ | $( $m:ident ),+ | { $( $body:tt )+ } $( $rest:tt )* }]
    } => {

        $crate::tt_call::tt_call! {
            macro = [{ private_parse_fresh }]
            input = [{ | $( $m ),+ | { $( $body )+ } $( $rest )* }]
            ~~> private_parse_clause! {
                $caller
            }
        }
    };

    // Special syntax for equality: x == y
    {
        $caller:tt
        input = [{ $left:tt == $right:tt $( $rest:tt )* }]
    } => {
        $crate::tt_call::tt_call! {
            macro = [{ private_parse_relation }]
            input = [{ $crate::relation::eq::eq ( $left, $right ) $( $rest )* }]
            ~~> private_parse_clause! {
                $caller
            }
        }
    };

    // Special syntax for disequality: x != y
    {
        $caller:tt
        input = [{ $left:tt != $right:tt $( $rest:tt )* }]
    } => {
        $crate::tt_call::tt_call! {
            macro = [{ private_parse_relation }]
            input = [{ $crate::relation::diseq::diseq ( $left, $right ) $( $rest )* }]
            ~~> private_parse_clause! {
                $caller
            }
        }
    };

    // Special syntax for succeeding goal: true
    {
        $caller:tt
        input = [{ true $( $rest:tt )* }]
    } => {
        $crate::tt_call::tt_call! {
            macro = [{ private_parse_relation }]
            input = [{ $crate::relation::succeed::succeed ( ) $( $rest )* }]
            ~~> private_parse_clause! {
                $caller
            }
        }
    };

    // Special syntax for failing goal: false
    {
        $caller:tt
        input = [{ false $( $rest:tt )* }]
    } => {
        $crate::tt_call::tt_call! {
            macro = [{ private_parse_relation }]
            input = [{ $crate::relation::fail::fail ( ) $( $rest )* }]
            ~~> private_parse_clause! {
                $caller
            }
        }
    };

    // [ conjunction of clauses ]
    {
        $caller:tt
        input = [{ [ $( $conjunction_list:tt )+ ] $( $rest:tt )* }]
    } => {

        $crate::tt_call::tt_call! {
            macro = [{ private_parse_conjunction_list }]
            input = [{ [ $( $conjunction_list )+ ] $( $rest )* }]
            ~~> private_parse_clause! {
                $caller
            }
        }
    };

    // $relation ( param1, param2, .. )
    {
        $caller:tt
        input = [{ $( $relation:ident )::+ ( $( $param:tt )* ) $( $rest:tt )* }]
    } => {
        $crate::tt_call::tt_call! {
            macro = [{ private_parse_relation }]
            input = [{ $( $relation )::+ ( $( $param )* ) $( $rest )* }]
            ~~> private_parse_clause! {
                $caller
            }
        }
    };

    // closure { conjunction of clauses }
    {
        $caller:tt
        input = [{ closure { $( $body:tt )+ } $( $rest:tt )* }]
    } => {
        $crate::tt_call::tt_call! {
            macro = [{ private_parse_closure }]
            input = [{ closure { $( $body )+ } $( $rest )* }]
            ~~> private_parse_clause! {
                $caller
            }
        }
    };

    // $operator { (dis)junction of clauses }
    {
        $caller:tt
        input = [{ $( $operator:ident )::+ { $( $body:tt )+ } $( $rest:tt )* }]
    } => {
        $crate::tt_call::tt_call! {
            macro = [{ private_parse_operator }]
            input = [{ $( $operator )::+ { $( $body )+ } $( $rest )* }]
            ~~> private_parse_clause! {
                $caller
            }
        }
    };

    // fngoal |state| { rust code }
    {
        $caller:tt
        input = [{ fngoal |$state:ident| { $( $body:tt )+ } $( $rest:tt )* }]
    } => {
        $crate::tt_call::tt_return! {
            $caller
            output = [{ $crate::operator::fngoal::FnGoal::new(Box::new(|$state| { $( $body )+ } )) }]
            rest = [{ $( $rest )* }]
        }
    };

    // fngoal move |state| { rust code }
    {
        $caller:tt
        input = [{ fngoal move |$state:ident| { $( $body:tt )+ } $( $rest:tt )* }]
    } => {
        $crate::tt_call::tt_return! {
            $caller
            output = [{ $crate::operator::fngoal::FnGoal::new(Box::new(move |$state| { $( $body )+ } )) }]
            rest = [{ $( $rest )* }]
        }
    };

    {
        $caller:tt
        input = [{ $e:expr , $( $rest:tt )* }]
    } => {
        $crate::tt_call::tt_return! {
            $caller
            output = [{ $e }]
            rest = [{ , $( $rest )* }]
        }
    };

    {
        $caller:tt
        input = [{ $e:expr }]
    } => {
        $crate::tt_call::tt_return! {
            $caller
            output = [{ $e }]
            rest = [{ }]
        }
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! private_proto_vulcan {
    {
        output = [{ $( $element:tt )* }]
        // only single clause allowed at the root of the body
        rest = [{ }]
    } => {
        $( $element )*
    };
}

#[macro_export]
macro_rules! proto_vulcan {
    ( $( $input:tt )* ) => {
        $crate::tt_call::tt_call! {
            macro = [{ private_parse_clause }]
            input = [{ $( $input )* }]
            ~~> private_proto_vulcan!{}
        }
    };
}
