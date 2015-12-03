require 'ffi'

UTF8 = 'UTF-8'.freeze
NAME_PARTS = %w( surname given_name initials first_initial middle_initials middle_names suffix display_short display_full ).freeze

module NativeHumanName
  extend FFI::Library
  ffi_lib 'target/release/libhuman_name.dylib'
  attach_function :human_name_parse, [:string], :pointer
  attach_function :human_name_consistent_with, [:pointer, :pointer], :bool
  attach_function :human_name_hash, [:pointer], :uint64

  NAME_PARTS.each do |part|
    attach_function "human_name_#{part}".to_sym, [:pointer], :pointer
  end

  attach_function :human_name_goes_by_middle_name, [:pointer], :bool
  attach_function :human_name_free_name, [:pointer], :void
  attach_function :human_name_free_string, [:pointer], :void
end

class ForeignString < String
  def self.wrap(pointer)
    new(pointer) unless pointer.null?
  end

  def initialize(pointer)
    @pointer = FFI::AutoPointer.new(pointer, NativeHumanName.method(:human_name_free_string))
    super(@pointer.read_string.force_encoding(UTF8))
  end
end

class HumanName < FFI::AutoPointer
  def self.parse(string)
    string = string.encode(UTF8) unless string.encoding == UTF8
    pointer = NativeHumanName.human_name_parse(string)
    new(pointer) unless pointer.null?
  end

  def initialize(pointer)
    super(pointer, NativeHumanName.method(:human_name_free_name))
  end

  def ==(other)
    other.is_a?(HumanName) && NativeHumanName.human_name_consistent_with(self, other)
  end

  def hash
    NativeHumanName.human_name_hash(self)
  end

  NAME_PARTS.each do |part|
    native_method = "human_name_#{part}".to_sym

    define_method part do
      pointer = NativeHumanName.send(native_method, self)
      ForeignString.wrap(pointer)
    end
  end
end

def Process.rss
  `ps -o rss= -p #{Process.pid}`.chomp.to_i
end

a = HumanName.parse("John Doe")
b = HumanName.parse("Jane Doe")
c = HumanName.parse("J. Doe")
d = HumanName.parse("John Allen Q. de la MacDonald, Jr.")
e = HumanName.parse("nope")

puts "failed parse is nil? #{e.nil?}"

puts "a == b: #{a == b}"
puts "a == c: #{a == c}"

puts "surnames: #{[a,b,c,d].map(&:surname)}"
puts "given names: #{[a,b,c,d].map(&:given_name)}"
puts "initials: #{[a,b,c,d].map(&:initials)}"
puts "first initial: #{[a,b,c,d].map(&:first_initial)}"
puts "middle initials: #{[a,b,c,d].map(&:middle_initials)}"
puts "middle names: #{[a,b,c,d].map(&:middle_names)}"
puts "suffixes: #{[a,b,c,d].map(&:suffix)}"
puts "display short: #{[a,b,c,d].map(&:display_short)}"
puts "display full: #{[a,b,c,d].map(&:display_full)}"
puts "unique hashes: #{[a,b,c,d].map(&:hash).uniq.count}"

puts "RSS (before) #{Process.rss}"

100000.times do
  n = HumanName.parse("John Doe")
  NAME_PARTS.each { |part| n.send(part) }
end

GC.start
puts "RSS (after) #{Process.rss}"
