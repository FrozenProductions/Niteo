#!/usr/bin/env ruby
# frozen_string_literal: true

formula_path = ARGV.fetch(0)
source_url = ENV.fetch("NITEO_HOMEBREW_SOURCE_URL")
source_sha256 = ENV.fetch("NITEO_HOMEBREW_SOURCE_SHA256")

formula = File.read(formula_path)
formula = formula.gsub(%r{url "https://github\.com/FrozenProductions/Niteo/archive/refs/tags/v[^"]+\.tar\.gz"},
                       %(url "#{source_url}"))
formula = formula.gsub(/sha256 "[0-9a-f]{64}"/, %(sha256 "#{source_sha256}"))
formula = formula.gsub(/\n  bottle do\n.*?\n  end\n/m, "\n")

File.write(formula_path, formula)
