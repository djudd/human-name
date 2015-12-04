require 'ffi'
require 'humanname/version'

module HumanName
  UTF8 = 'UTF-8'.freeze
  NAME_PARTS = %w( surname given_name initials first_initial middle_initials middle_names suffix display_short display_full ).freeze

  module Native
    extend FFI::Library
    ffi_lib File.expand_path('../libhuman_name.so', __FILE__)

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

  class NativeString < String
    def self.wrap(pointer)
      new(pointer) unless pointer.null?
    end

    def initialize(pointer)
      @pointer = FFI::AutoPointer.new(pointer, Native.method(:human_name_free_string))
      super(@pointer.read_string.force_encoding(UTF8))
    end
  end

  class Name < FFI::AutoPointer
    def self.parse(string)
      string = string.encode(UTF8) unless string.encoding == UTF8
      pointer = Native.human_name_parse(string)
      new(pointer) unless pointer.null?
    end

    def initialize(pointer)
      super(pointer, Native.method(:human_name_free_name))
    end

    def ==(other)
      other.is_a?(Name) && Native.human_name_consistent_with(self, other)
    end
    alias_method :eql?, :==

    def hash
      Native.human_name_hash(self)
    end

    NAME_PARTS.each do |part|
      native_method = "human_name_#{part}".to_sym

      define_method part do
        pointer = Native.send(native_method, self)
        NativeString.wrap(pointer)
      end
    end
  end

  def self.parse(string)
    Name.parse(string)
  end
end
