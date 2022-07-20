#!/usr/bin/ruby

lines = File.readlines(File.join(__dir__, '../tests/parseable-names.txt'))
names = lines.
  select {|l| !l.strip.empty? && !l.start_with?('#')}.
  map {|l| l.split('|', 2).first}

outdir = File.join(__dir__, './corpus/parse')
names.each_with_index do |name, i|
  fname = File.join(outdir, "#{i}.txt")
  File.write(fname, name)
end

lines = File.readlines(File.join(__dir__, '../tests/equal-names.txt'))
pairs = lines.
  select {|l| !l.strip.empty? && !l.start_with?('#')}.
  map {|l| l.split('|', 3).first(2)}
  
outdir = File.join(__dir__, './corpus/consistent_with')
pairs.each_with_index do |(a, b), i|
  fname = File.join(outdir, "#{i}.txt")
  File.write(fname, "#{a}#{b}")
end
