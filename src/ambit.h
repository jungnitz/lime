#pragma once

#include "eggmock.h"
#include "utils.h"

#include <mockturtle/networks/mig.hpp>

#include <cstdint>
#include <utility>

extern "C"
{
  struct ambit_compiler_statistics
  {
    uint64_t egraph_classes;
    uint64_t egraph_nodes;
    uint64_t egraph_size;

    uint64_t instruction_count;

    uint64_t t_runner;
    uint64_t t_extractor;
    uint64_t t_compiler;
  };

  struct ambit_compiler_settings
  {
    bool print_program;
    bool verbose;
    bool preoptimize = true;
    bool rewrite = true;
  };

  struct ambit_compiler_settings_ffi
  {
    bool print_program;
    bool verbose;
    bool rewrite = true;

    ambit_compiler_settings_ffi( ambit_compiler_settings s )
        : print_program( s.print_program ), verbose( s.verbose ), rewrite( s.rewrite ) {}
  };

  eggmock::mig_receiver<ambit_compiler_statistics> ambit_compile_ffi(
      ambit_compiler_settings_ffi settings );
  eggmock::mig_receiver<ambit_compiler_statistics> ambit_rewrite_ffi(
      ambit_compiler_settings_ffi settings,
      eggmock::mig_receiver<void> receiver );
}

inline std::pair<mockturtle::mig_network, ambit_compiler_statistics> ambit_rewrite(
    ambit_compiler_settings settings,
    mockturtle::mig_network& ntk )
{
  if ( settings.preoptimize )
  {
    preoptimize_mig( ntk );
  }
  mockturtle::mig_network out;
  const auto stat = eggmock::send_mig(
      ntk, ambit_rewrite_ffi( settings, eggmock::receive_mig( out ) ) );
  return { out, stat };
}

inline ambit_compiler_statistics ambit_compile(
    ambit_compiler_settings settings,
    mockturtle::mig_network& ntk )
{
  if ( settings.preoptimize )
  {
    preoptimize_mig( ntk );
  }
  mockturtle::mig_network out;
  const auto stat = eggmock::send_mig( ntk, ambit_compile_ffi( settings ) );
  return stat;
}
