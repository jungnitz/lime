/**
 * Runs compilation for given logic network (see `utils.hpp` for pre-provided logic networks)
 */
#include "fcdram.h"
#include "eggmock.h"
#include "utils.h"

#include <chrono>
#include <iostream>
#include <mockturtle/networks/aig.hpp>

using namespace mockturtle;
using namespace eggmock;
using namespace std::chrono;

// usage: exec [network]
int main( int const argc, char** argv )
{
  if ( argc != 2 )
  {
    std::cerr << "usage: " << argv[0] << std::endl;
    return 1;
  }

  std::optional<aig_network> aig = get_ntk<aig_network>( argv[1] );
  if ( !aig )
  {
    return 1;
  }

  auto const pre_opt_size = aig->size();

  auto const opt_begin = system_clock::now();
  preoptimize_aig( *aig );
  auto const t_opt = duration_cast<milliseconds>( system_clock::now() - opt_begin ).count();

  auto constexpr settings = fcdram_compiler_settings{
      .print_program = false,
      .verbose = false,
  };

  const auto [egraph_classes, egraph_nodes, egraph_size,
              instruction_count,
              t_runner, t_extractor, t_compiler] =
      send_aig( *aig, fcdram_compile( settings ) );

  std::cout << t_opt << "\t" << t_runner << "\t" << t_extractor << "\t" << t_compiler << "\t"
            << pre_opt_size << "\t" << aig->size() << "\t" << aig->num_cis() << "\t" << aig->num_cos() << "\t"
            << instruction_count << "\t"
            << egraph_classes << "\t" << egraph_nodes << "\t" << egraph_size;
  return 0;
}
