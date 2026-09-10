#!/usr/bin/env ruby
# frozen_string_literal: true

# Psych adapter: Ruby's YAML, and a libyaml front end.
#
# Psych drops comments like PyYAML does, so it answers `load` and
# `roundtrip` only. Its value is that it is a second independent judge of
# whether a document is well formed, over the same libyaml many other
# tools embed.
require 'psych'
require 'json'

def canon(v)
  case v
  when Hash then v.to_h { |k, x| [k.to_s, canon(x)] }
  when Array then v.map { |x| canon(x) }
  when String, Integer, Float, TrueClass, FalseClass, NilClass then v
  else v.to_s
  end
end

# `unsafe_load` arrived in Psych 3.3; older rubies only have `load`, which
# had the same permissive behaviour then. Aliases and merge keys are the
# point of several cases, so the permissive entry point is the right one.
def load_any(src)
  Psych.respond_to?(:unsafe_load) ? Psych.unsafe_load(src) : Psych.load(src)
end

req = JSON.parse($stdin.read)
op = req['op']
out =
  if op == 'version'
    { 'version' => "Psych #{Psych::VERSION} (libyaml #{Psych::LIBYAML_VERSION})" }
  else
    begin
      case op
      when 'load' then { 'result' => JSON.generate(canon(load_any(req['source']))) }
      when 'roundtrip' then { 'result' => Psych.dump(load_any(req['source'])) }
      when 'comments', 'delete', 'set_comment'
        { 'unsupported' => 'Psych keeps no comments and has no editing model' }
      else { 'error' => "unknown op #{op}" }
      end
    rescue StandardError => e
      { 'error' => "#{e.class}: #{e.message.lines.first.to_s.strip}" }
    end
  end
puts JSON.generate(out)
