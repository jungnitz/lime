#pragma once

#include <iostream>
#include <lorina/aiger.hpp>
#include <lorina/common.hpp>
#include <lorina/pla.hpp>
#include <lorina/verilog.hpp>
#include <mockturtle/generators/arithmetic.hpp>
#include <mockturtle/io/aiger_reader.hpp>
#include <mockturtle/io/pla_reader.hpp>
#include <mockturtle/io/verilog_reader.hpp>

#include <optional>
#include <string>
#include <string_view>

template<class ntk>
std::optional<ntk> get_ntk( std::string const& key );

template<class ntk>
std::optional<ntk> read_ntk( std::string const& path );

void preoptimize_mig( mockturtle::mig_network& ntk );

/**
 * Collection of logic networks (eg for benchmarking)
 * - included logic networks: Full Adder ("fa"), Multiplexer ("mux"), Greater than ("gt"), "kogge_stone"
 * - carry_ripple_adder_inplace ("add"), carry_ripple_multiplier ("mul"), sum-adder ("pop")
 */
template<class ntk_t>
std::optional<ntk_t> get_ntk( std::string const& key )
{
  // Read from file
  if ( key.find( '.' ) != std::string::npos )
  {
    return read_ntk<ntk_t>( key );
  }

  // Full adder
  if ( key == "fa" )
  {
    ntk_t ntk;
    const auto in1 = ntk.create_pi();
    const auto in2 = ntk.create_pi();
    const auto in3 = ntk.create_pi();
    const auto [s, c] = mockturtle::full_adder( ntk, in1, in2, in3 );
    ntk.create_po( s );
    ntk.create_po( c );
    return ntk;
  }

  if (key == "mux")
  {
    ntk_t ntk;

    const auto b_i = ntk.create_pi();
    const auto b_i_next = ntk.create_pi();
    const auto m = ntk.create_pi();

    const auto O1 = ntk.create_and( m, b_i_next );
    const auto O2 = ntk.create_and( ntk.create_not( m ), b_i );
    const auto bi = ntk.create_or( O1, O2 );
    ntk.create_po( bi );

    return ntk;
  }

  if ( key == "gt" )
  {
    ntk_t ntk;

    const auto a = ntk.create_pi();
    const auto b = ntk.create_pi();
    const auto c = ntk.create_pi();
    const auto f = ntk.create_pi();

    const auto flag = ntk.create_xor( a, b );
    const auto gt = ntk.create_and( ntk.create_not( f ), ntk.create_and( a, ntk.create_not( b ) ) );
    const auto found = ntk.create_or( flag, f );
    const auto cmp = ntk.create_or( c, gt );

    ntk.create_po( cmp );
    ntk.create_po( found );

    return ntk;
  }

  if ( key == "kogge_stone" )
  {
    ntk_t ntk;

    // Create primary inputs
    const auto A = ntk.create_pi();
    const auto B = ntk.create_pi();

    // Compute intermediate signals
    const auto X = ntk.create_xor( A, B );
    const auto P0 = ntk.create_or( A, B );
    const auto G0 = ntk.create_and( A, B );

    ntk.create_po( P0 );
    const auto shifted_P0 = ntk.create_pi();
    const auto P1 = ntk.create_and( P0, shifted_P0 );

    ntk.create_po( G0 );
    const auto shifted_G0 = ntk.create_pi();
    const auto G1 = ntk.create_or( G0, ntk.create_and( P0, shifted_G0 ) );

    ntk.create_po( G1 );
    const auto shifted_G1 = ntk.create_pi();
    ntk.create_po( P1 );
    const auto shifted_P1 = ntk.create_pi();
    const auto P2 = ntk.create_and( P1, shifted_P1 );
    const auto G2 = ntk.create_or( G1, ntk.create_and( P1, shifted_G1 ) );

    // const auto P3 = ntk.create_and(P2, ntk.create_shift(P2));
    ntk.create_po( G2 );
    const auto shifted_G2 = ntk.create_pi();
    const auto G3 = ntk.create_or( G2, ntk.create_and( P2, shifted_G2 ) );

    const auto Carry = G3;
    const auto shifted_G3 = ntk.create_pi();
    const auto S = ntk.create_xor( X, shifted_G3 );

    // Create primary outputs
    ntk.create_po( S );
    ntk.create_po( Carry );

    return ntk;
  }

  bool const is_add = key.starts_with( "add" );
  bool const is_mul = key.starts_with( "mul" );
  bool const is_pop = key.starts_with( "pop" );
  if ( is_add || is_mul || is_pop )
  {
    unsigned long n;
    try
    {
      auto const rest = std::string_view( key ).substr( 3 );
      n = std::stoul( rest.data() );
    }
    catch ( std::exception& )
    {
      std::cerr << "invalid number given in '" << key << "'" << std::endl;
      return {};
    }

    ntk_t ntk;
    auto const gen_inputs = [&] {
      std::vector<typename ntk_t::signal> signals;
      signals.reserve( n );
      for ( unsigned long i = 0; i < n; i++ )
      {
        signals.emplace_back( ntk.create_pi() );
      }
      return signals;
    };

    std::vector<typename ntk_t::signal> outputs;
    if ( is_add )
    {
      auto carry = ntk.get_constant( false );
      outputs = gen_inputs();
      mockturtle::carry_ripple_adder_inplace( ntk, outputs, gen_inputs(), carry );
    }
    else if ( is_mul )
    {
      outputs = mockturtle::carry_ripple_multiplier( ntk, gen_inputs(), gen_inputs() );
    }
    else if ( is_pop )
    {
      outputs = mockturtle::sideways_sum_adder( ntk, gen_inputs() );
    }
    else
    {
      throw;
    }

    for ( auto const& sig : outputs )
    {
      ntk.create_po( sig );
    }
    return ntk;
  }

  std::cerr << "invalid network '" << key << "'" << std::endl;
  return {};
}

/**
 * Read network from .aig/.pla/.verilog file
 */
template<class ntk_t>
std::optional<ntk_t> read_ntk( const std::string& path )
{
  ntk_t ntk;
  lorina::return_code res;
  if ( path.ends_with( ".aig" ) )
  {
    res = lorina::read_aiger( path, mockturtle::aiger_reader( ntk ) );
  }
  else if ( path.ends_with( ".pla" ) )
  {
    res = lorina::read_pla( path, mockturtle::pla_reader( ntk ) );
  }
  else if ( path.ends_with( ".verilog" ) )
  {
    res = lorina::read_verilog( path, mockturtle::verilog_reader( ntk ) );
  }
  else
  {
    std::cerr << "unknown file ending (file " << path << ")" << std::endl;
    return {};
  }
  if ( res != lorina::return_code::success )
  {
    std::cerr << "could not parse file " << path << std::endl;
    return {};
  }
  return ntk;
}
