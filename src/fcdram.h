#pragma once

#include "eggmock.h"

#include <cstdint>
#include <string>

extern "C"
{
  struct fcdram_compiler_statistics
  {
    uint64_t egraph_classes;
    uint64_t egraph_nodes;
    uint64_t egraph_size;

    uint64_t instruction_count;
  };

  /**
   * @param print_compilation_stats Whether to print stats like `t_runner`,`t_extractor`,`t_compiler`
   */
  struct fcdram_compiler_settings
  {
    bool print_program;
    bool verbose;
	bool print_compilation_stats;
	double min_success_rate;
    uint64_t repetition_fracops;
	uint8_t safe_space_rows_per_subarray;
	const char *config_file;
	bool do_save_config;
  };

  eggmock::aig_receiver<eggmock::mig_rewrite> fcdram_rewriter( fcdram_compiler_settings settings );
  eggmock::aig_receiver<fcdram_compiler_statistics> fcdram_compile( fcdram_compiler_settings settings );
  // void fcdram_compile();
}
