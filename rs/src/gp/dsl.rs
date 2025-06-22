macro_rules! define_gp_architecture {
    ($name:ident $tt:tt) => {};
}

define_gp_architecture! {
    Ambit {
        cells ([T; 4], [DCC; 2], [D]),
        operands (
            DCC = [(DCC), (!DCC)],
            ANY = [(T), (D), ...DCC],
            ANY_AND_CONST = [(1), (0), ...ANY],
            TRA = [
                (T[0], T[1], T[2]),
                (T[1], T[2], T[3]),
                (T[0], T[1], !DCC[0]),
            ],
            MRA = [
                (T[0], T[1]),
                (T[2], T[3]),
                ...TRA
            ],
            OUTPUTS = [
                (),
                ...TRA,
                ...ANY
            ]
        ),
        operations (
            TRA(TRA = maj) -> maj,
            RC(ANY_AND_CONST) -> and
        ),
        outputs (OUTPUTS)
    }
}

define_gp_architecture! {
    IMPLY {
        cells([D]),
        operands (
            PAIR = [
                (D, !D),
                (_, !D),
            ],
            ANY = [(D)]
        ),
        operations (
            IMP(PAIR = (_, and)),
            FALSE(ANY = false)
        )
    }
}

define_gp_architecture! {
    PLiM {
        cells([D]),
        operands (
            TRIPLET = [
                (D, !D, D),
                (_, _, D),
            ],
        ),
        operations (
            RMA3(TRIPLET = (_, _, maj))
        )
    }
}

define_gp_architecture! {
    FELIX {
        cells([D]),
        operands (
            ANY: [(D)],
            NARY: [D]
            TRIPLET: [(D, D, D)],
            PAIR: [(D, D)],
        ),
        operations (
            // or
            OR(NARY -> or),
            // nor
            NOR(NARY -> !or),

            NAND2(PAIR -> !and),
            NAND3(TRIPLET -> !and),

            MIN(TRIPLET -> !maj),
        ),
        outputs (ANY)
    }
}
