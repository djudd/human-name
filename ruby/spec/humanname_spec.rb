require 'spec_helper'

describe HumanName do
  describe 'parse' do
    it 'parses simple name' do
      n = HumanName.parse("Jane Doe")
      expect(n.given_name).to eq('Jane')
      expect(n.surname).to eq('Doe')
      expect(n.middle_names).to be_nil
      expect(n.suffix).to be_nil
      expect(n.display_full).to eq('Jane Doe')
    end

    it 'parses complex name' do
      n = HumanName.parse("JOHN ALLEN Q DE LA MACDONALD JR")
      expect(n.given_name).to eq('John')
      expect(n.surname).to eq('de la MacDonald')
      expect(n.middle_names).to eq('Allen')
      expect(n.suffix).to eq('Jr.')
      expect(n.display_full).to eq('John Allen Q. de la MacDonald, Jr.')
    end

    it 'returns nil on failure' do
      expect(HumanName.parse 'nope').to be_nil
    end
  end

  describe '==' do
    it 'is true for identical names' do
      expect(HumanName.parse "Jane Doe").to eq(HumanName.parse "Jane Doe")
    end

    it 'is true for consistent but non-identical names' do
      expect(HumanName.parse "Jane Doe").to eq(HumanName.parse "J. Doe")
    end

    it 'is false for inconsistent names' do
      expect(HumanName.parse "Jane Doe").not_to eq(HumanName.parse "John Doe")
    end
  end

  describe 'hash' do
    it 'is identical for identical names' do
      expect(HumanName.parse("Jane Doe").hash).to eq(HumanName.parse("Jane Doe").hash)
    end

    it 'is identical for consistent names' do
      expect(HumanName.parse("Jane Doe").hash).to eq(HumanName.parse("J. Doe").hash)
    end

    it 'is different for names with different surnames' do
      expect(HumanName.parse("Jane Doe").hash).not_to eq(HumanName.parse("J. Dee").hash)
    end
  end

  it 'does not leak memory' do
    def rss
      `ps -o rss= -p #{Process.pid}`.chomp.to_i
    end

    before = rss

    100000.times do
      n = HumanName.parse("John Doe")
      HumanName::NAME_PARTS.each { |part| n.send(part) }
    end

    expect(rss).to be < 2 * before
  end
end
