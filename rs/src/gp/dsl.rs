macro_rules! define_gp_architecture {
    ($name:ident $tt:tt) => {};
}

define_gp_architecture! {
    Ambit {
        cells ([T; 4], [DCC; 2], [D]),
        operand_sets (
            DCC = [ (DCC), (!DCC) ],
            any = [ (T), (D), ...DCC ],
            any_and_const = [0, 1, ...any]
            TRA = [
                (T0, T1, T2),
                (T1, T2, T3),
                (T0, T1, !DCC0),
            ],,
            MRA = [
                (T0, T1),
                (T2, T3),
                ...TRA
            ]
        ),
        operations (
            TRA = maj(...) -> maj(...),
            any_and_const -> and(.0)
        ),
        outputs (
            (),
            any,
            MRA,
        )
    }
}

define_gp_architecture! {
    IMPLY {
        cells([D]),
        operand_sets (
            imp = [(D * D)]
        ),
        operations (
            imp = (.0, !and(.0, !.1))
        )
    }
}

define_gp_architecture! {
    PLiM {
        cells([D]),
        operand_sets (
            triple: [(D * D * D)],
        ),
        operations (
            triple = (.0, .1, maj(.0, !.1, .2))
        )
    }
}

define_gp_architecture! {
    FELIX {
        cells([D]),
        operand_sets (
            any: [(D)],
            nary: [(D*)]
            three: [(D * D * D)],
            two: [(D * D)],
        ),
        operations (
            // or
            nary -> !and(!...),
            // nor
            nary -> and(!...)

            two -> !and(...),
            three -> !and(...),

            three -> !maj(...),
        ),
        outputs (any)
    }
}
