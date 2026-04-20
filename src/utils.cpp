#include "utils.h"

#include <mockturtle/algorithms/cleanup.hpp>
#include <mockturtle/algorithms/functional_reduction.hpp>
#include <mockturtle/algorithms/mig_algebraic_rewriting.hpp>
#include <mockturtle/algorithms/mig_inv_optimization.hpp>
#include <mockturtle/algorithms/mig_inv_propagation.hpp>
#include <mockturtle/algorithms/mig_resub.hpp>
#include <mockturtle/algorithms/resubstitution.hpp>

using namespace mockturtle;

void preoptimize_mig( mockturtle::mig_network& ntk )
{
  depth_view depth_mig{ ntk };
  fanout_view fanout_mig{ depth_mig };
  resubstitution_params ps;
  resubstitution_stats st;

  functional_reduction( fanout_mig );
  mig_inv_optimization( fanout_mig );
  mig_resubstitution2( fanout_mig, ps, &st );
  // mig_resubstitution( fanout_mig,  ps, &st  );

  mig_algebraic_depth_rewriting( depth_mig );
  ntk = cleanup_dangling( ntk );
}
