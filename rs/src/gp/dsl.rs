use std::thread;

#[macro_export]
macro_rules! define_gp_architecture {
    ($vis:vis $name:ident {
        cells ($($cell:tt),*),
        operands (
            $($op_name:ident = $op:tt),*
            $(,)?
        ),
        operations $operations:tt
        $(, outputs $outputs:tt)?
    }) => {
        $crate::paste::paste! {
            $vis struct $name {
            }
            impl $crate::gp::Architecture for $name {
                type CellType = [< $name CellType >];
                type Cell = [< $name Cell >];

                fn outputs(&self) -> $crate::gp::Operands<Self> {
                    todo!()
                }
                fn operations(&self) -> &'static [$crate::gp::OperationType<Self>] {
                    todo!()
                }
            }
            $(
                static [< $name:upper _OPERAND_ $op_name >]: std::sync::LazyLock< $crate::gp::Operands<$name> > = std::sync::LazyLock::new(|| {
                    $crate::define_gp_architecture!(@operands $name $op)
                });
            )*
        }
        $crate::define_gp_architecture!(@cell_types $vis $name $($cell),*);
    };

    // Cell & CellType implementation
    (@cell_types $vis:vis $arch_name:ident $([$name:ident $(; $num:literal)?]),*) => {
        $crate::paste::paste! {
            #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
            $vis enum [< $arch_name CellType >] {
                Constant,
                $(
                    $name
                ),*
            }
            impl $crate::gp::CellType for [< $arch_name CellType >] {
                type Arch = $arch_name;
                const CONSTANT: Self = Self::Constant;
                fn count(self) -> Option<$crate::gp::CellIndex> {
                    match self {
                        Self::Constant => Some(2),
                        $(
                            Self::$name => $crate::define_gp_architecture!(@optional $($num)?)
                        ),*
                    }
                }
                fn name(self) -> &'static str {
                    match self {
                        Self::Constant => "bool",
                        $(
                            Self::$name => stringify!($name)
                        ),*
                    }
                }
            }
            impl std::fmt::Display for [< $arch_name CellType >] {
                fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
                    write!(f, "{}", $crate::gp::CellType::name(*self))
                }
            }

            #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
            $vis enum [< $arch_name Cell >] {
                Constant($crate::gp::CellIndex),
                $(
                    $name($crate::gp::CellIndex)
                ),*
            }
            impl $crate::gp::Cell for [< $arch_name Cell >] {
                type Arch = $arch_name;

                fn new(typ: [< $arch_name CellType >], idx: $crate::gp::CellIndex) -> Self {
                    debug_assert!(
                        $crate::gp::CellType::count(typ).is_none_or(|count| count > idx),
                        "cell index out of bounds"
                    );
                    use [< $arch_name CellType >]::*;
                    match typ {
                        Constant => Self::Constant(idx),
                        $(
                            $name => Self::$name(idx)
                        ),*
                    }
                }

                fn typ(self) -> [< $arch_name CellType >] {
                    use [< $arch_name CellType >]::*;
                    match self {
                        Self::Constant(_) => Constant,
                        $(
                            Self::$name(_) => $name
                        ),*
                    }
                }

                fn index(self) -> $crate::gp::CellIndex {
                    match self {
                        Self::Constant(idx) => idx,
                        $(
                            Self::$name(idx) => idx
                        ),*
                    }
                }
            }
            impl std::fmt::Display for [< $arch_name Cell >] {
                fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
                    if let Some(v) = $crate::gp::Cell::constant_value(*self) {
                        write!(f, "{:?}", v)
                    } else {
                        write!(f, "{}[{}]", $crate::gp::Cell::typ(*self), $crate::gp::Cell::index(*self))
                    }
                }
            }
        }
    };

    // Tuple operands
    (@operands $arch_name:ident [
        $( ( $($op:tt)* ) ),*
        $(,)?
        $(
            ...$operand:ident
        ),*
        $(,)?
    ]) => {
        $crate::gp::Operands::tuples(
            [
                $(
                    $crate::define_gp_architecture!(
                        @operand_types $arch_name, []; $($op)*
                    ).as_slice()
                ),*
            ],
            $crate::paste::paste! {
                [
                    $(&*[< $arch_name:upper _OPERAND_ $operand >]),*
                ]
            }
        )
    };
    // Nary operands
    (@operands $arch_name:ident [! $name:ident]) => {
        $crate::gp::Operands::nary(
            $crate::define_gp_architecture!(@operand_type $arch_name, true $name)
        )
    };
    (@operands $arch_name:ident [$name:ident]) => {
        $crate::gp::Operands::nary(
            $crate::define_gp_architecture!(@operand_type $arch_name, false $name)
        )
    };

    // Parsing array of operand types
    // [OperandType< $arch_name >]: @operand_types $arch_name, []; DCC[0], T, ...
    (@operand_types $arch_name:ident, [ $($acc:tt)* ]; , $($tail:tt)*) => {
        $crate::define_gp_architecture!(
            @operand_types $arch_name, [ $($acc)* , ]; $($tail)*
        )
    };
    (@operand_types $arch_name:ident, [ $($acc:tt)* ];) => {
        [$($acc)*]
    };
    (@operand_types $arch_name:ident, [ $($acc:tt)* ]; ! $($tail:tt)*) => {
        $crate::define_gp_architecture!(
            @operand_types_inv $arch_name, [$($acc)*], true; $($tail)*
        )
    };
    (@operand_types $arch_name:ident, [ $($acc:tt)* ]; $($tail:tt)*) => {
        $crate::define_gp_architecture!(
            @operand_types_inv $arch_name, [ $($acc)* ], false; $($tail)*
        )
    };
    (@operand_types_inv $arch_name:ident, [ $($acc:tt)* ], $inverted:expr; $cell:ident [$idx:expr] $($tail:tt)*) => {
        $crate::define_gp_architecture!(
            @operand_types
            $arch_name,
            [
                $($acc)*
                $crate::define_gp_architecture!(@operand_type $arch_name, $inverted $cell $idx)
            ];
            $($tail)*
        )
    };

    (@operand_types_inv $arch_name:ident, [ $($acc:tt)* ], $inverted:literal; $cell:ident $($tail:tt)*) => {
        $crate::define_gp_architecture!(
            @operand_types
            $arch_name,
            [
                $($acc)*
                $crate::define_gp_architecture!(@operand_type $arch_name, $inverted $cell)
            ];
            $($tail)*
        )
    };

    // Parsing operand type
    // OperandType< $arch_name >: @operand_type [inverted: bool] [cell: ident] [idx: literal]
    (@operand_type $arch_name:ident, $inverted:literal true) => {
        $crate::paste::paste! {
            $crate::gp::OperandType::< $arch_name > {
                typ: [< $arch_name CellType >]::Constant,
                inverted: $inverted,
                index: Some(1),
            }
        }
    };
    (@operand_type $arch_name:ident, $inverted:literal false) => {
        $crate::paste::paste! {
            $crate::gp::OperandType::< $arch_name > {
                typ: [< $arch_name CellType >]::Constant,
                inverted: $inverted,
                index: Some(0),
            }
        }
    };
    (@operand_type $arch_name:ident, $inverted:literal bool) => {
        $crate::paste::paste! {
            $crate::gp::OperandType::< $arch_name > {
                typ: [< $arch_name CellType >]::Constant,
                inverted: $inverted,
                index: None,
            }
        }
    };
    (@operand_type $arch_name:ident, $inverted:literal $name:ident $($idx:expr)?) => {
        $crate::paste::paste! {
            $crate::gp::OperandType::< $arch_name > {
                typ: [< $arch_name CellType >]::$name,
                inverted: $inverted,
                index: $crate::define_gp_architecture!(@optional $($idx)?),
            }
        }
    };

    // Operations
    (
        @operation_types
        $arch_name:ident;
        $(
            $name:ident ( $ops:ident $(:=)? )
        )*
    ) => {

    };
    (@operation_type $arch_name:ident, $name:ident, $ops:ident, [$($acc:tt)*]; -> $($tail:tt)*) => {
        $crate::gp::OperationType::< $arch_name > {
            name: stringify!($name),
            input: $crate::paste::paste([< $arch_name:upper _OPERAND_ $ops >]).clone(),
            input_results: $crate::define_gp_architecture!(@input_results []; $($acc)*),
            output: $crate::define_gp_architecture!(@parse_opt_function $($tail)*),
        }
    };

    // Functions, Gates and InputResult
    // [InputResult]: @input_results []; _, _, !maj
    (@input_results [$($acc:tt)*];) => {
        [$($acc)*]
    };
    (@input_results [$($acc:tt)*]; , $($tail:tt)*) => {
        $crate::define_gp_architecture!(@input_results [$($acc)* ,]; $($tail)*)
    };
    (@input_results [$($acc:tt)*]; ! $gate:ident $($tail:tt)*) => {
        $crate::define_gp_architecture!(
            @input_results
            [
                $($acc)*
                $crate::gp::InputResult::Function(
                    $crate::define_gp_architecture!(@function true $gate)
                )
            ];
            $($tail)*
        )
    };
    (@input_results [$($acc:tt)*]; _ $($tail:tt)*) => {
        $crate::define_gp_architecture!(
            @input_results
            [
                $($acc)*
                $crate::gp::InputResult::Unchanged
            ];
            $($tail)*
        )
    };
    (@input_results [$($acc:tt)*]; $gate:ident $($tail:tt)*) => {
        $crate::define_gp_architecture!(
            @input_results
            [
                $($acc)*
                $crate::gp::InputResult::Function(
                    $crate::define_gp_architecture!(@function false $gate)
                )
            ];
            $($tail)*
        )
    };

    (@parse_opt_function ! $gate:ident) => { Some($crate::define_gp_architecture!(@function true $gate)) };
    (@parse_opt_function $gate:ident) => { Some($crate::define_gp_architecture!(@function false $gate)) };
    (@parse_opt_function) => { None };
    (@function $inverted:literal $gate:ident) => {
        $crate::gp::Function {
            gate: $crate::define_gp_architecture!(@gate $gate),
            inverted: $inverted,
        }
    };
    (@gate true) => { $crate::gp::Gate::Constant(true) };
    (@gate false) => { $crate::gp::Gate::Constant(false) };
    (@gate $name:ident) => { $crate::paste::paste!($crate::gp::Gate::[< $name:camel >]) };

    (@optional $value:expr) => { Some($value) };
    (@optional) => { None }
}

// define_gp_architecture! {
//     pub Ambit {
//         cells ([T; 4], [DCC; 2], [D]),
//         operands (
//             DCC = [(DCC), (!DCC)],
//             ANY = [(T), (D), ...DCC],
//             ANY_AND_CONST = [(bool), ...ANY],
//             TRA = [
//                 (T[0], T[1], T[2]),
//                 (T[1], T[2], T[3]),
//                 (T[0], T[1], !DCC[0])
//             ],
//             MRA = [
//                 (T[0], T[1]),
//                 (T[2], T[3]),
//                 ...TRA
//             ],
//             OUTPUTS = [
//                 (),
//                 ...TRA,
//                 ...ANY
//             ]
//         ),
//         operations (
//             TRA(TRA := maj -> maj),
//             RC(ANY_AND_CONST) -> and
//         ),
//         outputs (OUTPUTS)
//     }
// }
//
// define_gp_architecture! {
//     pub IMPLY {
//         cells([D]),
//         operands (
//             PAIR = [
//                 (D, !D),
//                 (bool, !D)
//             ],
//             ANY = [(D)]
//         ),
//         operations (
//             IMP(PAIR = (_, and)),
//             FALSE(ANY = false)
//         )
//     }
// }
//
// define_gp_architecture! {
//     pub PLiM {
//         cells([D]),
//         operands (
//             TRIPLET = [
//                 (D, !D, D),
//                 (bool, bool, D)
//             ]
//         ),
//         operations (
//             RMA3(TRIPLET := _, _, maj -> maj)
//         )
//     }
// }
//
// define_gp_architecture! {
//     pub FELIX {
//         cells([D]),
//         operands (
//             ANY = [(D)],
//             NARY = [D],
//             NOT_NARY = [!D],
//             TERNARY = [(D, D, D)],
//             BINARY = [(D, D)]
//         ),
//         operations (
//             // or
//             OR(NOT_NARY -> !and),
//             // nor
//             NOR(NOT_NARY -> and),
//
//             NAND2(BINARY -> !and),
//             NAND3(TERNARY -> !and),
//             MIN(TERNARY -> !maj),
//         ),
//         outputs (ANY)
//     }
// }
//
