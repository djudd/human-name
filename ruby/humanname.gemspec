require File.expand_path("../lib/#{File.basename(__FILE__, '.gemspec')}/version", __FILE__)

Gem::Specification.new do |s|
  s.name        = 'humanname'
  s.version     = HumanName::VERSION
  s.date        = '2015-12-03'
  s.summary     = "A library for parsing and comparing human names"
  s.description = "A library for parsing and comparing human names. Wraps the Rust crate `human_name`."
  s.authors     = ["David Judd"]
  s.email       = 'david.a.judd@gmail.com'
  s.files       = %w(humanname.gemspec Gemfile) + Dir.glob("{lib,spec}/**")
  s.homepage    = 'https://https://github.com/djudd/human-name/ruby'
  s.license     = 'Apache-2.0'
  s.platform    = Gem::Platform::CURRENT # Linux only, for now
  s.add_runtime_dependency 'ffi', '~> 1.9.1'
  s.add_development_dependency 'rake', '~> 10.4.2'
  s.add_development_dependency 'rspec', '~> 3.4.0'
  s.add_development_dependency 'rubygems-tasks', '~> 0.2.4'
end
