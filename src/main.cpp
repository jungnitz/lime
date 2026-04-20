#include "ambit.h"
#include "fcdram.h"

#include <mockturtle/io/write_dot.hpp>
#include <mockturtle/networks/mig.hpp>

using namespace mockturtle;
using namespace eggmock;

void run_ambit_example(mig_network in)
{

  write_dot( in, "in.dot" );

  const auto settings = ambit_compiler_settings{
      .print_program = true,
      .verbose = true,
      .preoptimize = true,
      .rewrite = true,
  };

  auto [out, result] = ambit_rewrite( settings, in );
  std::cout << "IC:" << result.instruction_count << std::endl;
  std::cout << "t1:" << result.t_runner << std::endl;
  std::cout << "t2:" << result.t_extractor << std::endl;
  std::cout << "t3:" << result.t_compiler << std::endl;

  write_dot( out, "out.dot" );
  // ambit_compiler_statistics result = eggmock::send_mig( in, ambit_compile(settings) );
}

/**
 * TODO: change `mig` to `aig`??
 */
void run_fcdram_example()
{

  aig_network in;
  // const auto b_i = in.create_pi();
  // const auto b_i_next = in.create_pi();
  // const auto m = in.create_pi();
  //
  // const auto O1 = in.create_and( m, b_i_next );
  // const auto O2 = in.create_and( in.create_not( m ), b_i );
  // const auto O3 = in.create_and( in.create_not( O2 ), O1 );
  // const auto bi = in.create_or( O1, O2 );
  // in.create_po( bi );
  // in.create_po( O3 );

  // test and(and2,and2) -> and4
  const auto i1 = in.create_pi();
  const auto i2 = in.create_pi();
  const auto i3 = in.create_pi();
  const auto i4 = in.create_pi();

  const auto o1 = in.create_and( i1, i2);
  const auto o2 = in.create_and( i3, i4);
  const auto o3 = in.create_and( o1,o2 );
  in.create_po( o3 );

  write_dot( in, "in.dot" );
  std::cout << "Sending graph to fcdram_compile..." << std::endl;
  // fcdram_compile();

  // use `eggmock` to send mockturtle-graph to `lime`'s entry point `fcdram_compile()`
  fcdram_compiler_statistics result = eggmock::send_aig( in, fcdram_compile( fcdram_compiler_settings{
                                                           .print_program = true,
                                                           .verbose = true,
														   .print_compilation_stats = true,
														   .min_success_rate= 99.9999,
    													   .repetition_fracops=5, // issue 5 FracOps per init of reference subarray
														   .safe_space_rows_per_subarray = 16,
														   .config_file = "",
														   .do_save_config = true,
                                                       } ) );
  // std::cout << "IC:" << result.instruction_count << std::endl;
  // std::cout << "t1:" << result.t_runner << std::endl;
  // std::cout << "t2:" << result.t_extractor << std::endl;
  // std::cout << "t3:" << result.t_compiler << std::endl;

  // aig_network rewritten = rewrite_mig( in, fcdram_rewriter() );
  // write_dot( rewritten, "out.dot" );
}

int main()
{
  mig_network in;
  const auto b_i = in.create_pi();
  const auto b_i_next = in.create_pi();
  const auto m = in.create_pi();

  const auto O1 = in.create_and( m, b_i_next );
  const auto O2 = in.create_and( in.create_not( m ), b_i );
  const auto bi = in.create_or( O1, O2 );
  in.create_po( bi );

  write_dot( in, "in.dot" );
  //run_ambit_example(in);
  run_fcdram_example();
}
