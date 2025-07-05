use super::define_generic_architecture;

define_generic_architecture! {
    pub Ambit {
        cells ([T; 4], [DCC; 2], [D]),
        operands (
            NONE = [()],
            DCC = [(DCC), (!DCC)],
            ANY = [(T), (D), ...DCC],
            ANY_AND_CONST = [(bool), ...ANY],
            TRA = [
                (T[0], T[1], T[2]),
                (T[1], T[2], T[3]),
                (T[0], T[1], !DCC[0])
            ],
            DRA = [
                (T[0], T[1]),
                (T[2], T[3]),
            ],
        ),
        operations (
            TRA = ([*] := maj(TRA)),
            RC = (and(ANY_AND_CONST))
        ),
        output (TRA, DRA, ANY, NONE)
    }
}

define_generic_architecture! {
    pub IMPLY {
        cells([D]),
        operands (
            PAIR = [
                (D, !D),
                (bool, !D)
            ],
            ANY = [(D)]
        ),
        operations (
            IMP = ([1] := and(PAIR)),
            FALSE = ([0] := false(ANY))
        )
    }
}

define_generic_architecture! {
    pub PLiM {
        cells([D]),
        operands (
            TRIPLET = [
                (D, !D, D),
                (bool, !D, D),
                (D, !bool, D),
                (bool, !bool, D)
            ]
        ),
        operations (
            RMA3 = ([2] := maj(TRIPLET))
        )
    }
}

define_generic_architecture! {
    pub FELIX {
        cells([D]),
        operands (
            ANY = [(D)],
            NARY = [D | bool],
            NOT_NARY = [!D | !bool],
            TERNARY = [(D, D, D), (bool, D, D), (bool, bool, D), (bool, bool, bool)],
            BINARY = [(D, D), (bool, D), (bool, bool)]
        ),
        operations (
            // or
            OR = (!and(NOT_NARY)),
            // nor
            NOR = (and(NOT_NARY)),

            NAND2 = (!and(BINARY)),
            NAND3 = (!and(TERNARY)),
            MIN = (!maj(TERNARY)),
        ),
        output (ANY)
    }
}
