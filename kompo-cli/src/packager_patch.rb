FS_LOAD_PATHS = Fs.get_load_paths
AUTOLOAD_MAP = {}
EXTS=['.rb','.so']
class Module
  alias original_autoload autoload
  def autoload(const, file)
    # p "module autoload: #{self}"
    AUTOLOAD_MAP[file] = [self, const, false]
    original_autoload(const, file)
  end
end
def autoload(const, file)
  # p "kernel autoload"
  Object.autoload(const, file)
end
alias original_caller_locations caller_locations
def caller_locations(a=1,b=nil)
  c=original_caller_locations(1..-3).delete_if{_1.to_s.match?(/^-e:/)}
  return c[a] if a.is_a?(Range)
  b.nil? ? c[a..b]:c[a,b]
end
alias original_caller caller
def caller(a=1,b=nil)
  c=original_caller(1..-3).delete_if{_1.match?(/^-e:/)}
  return c[a] if a.is_a?(Range)
  b.nil? ? c[a..b]:c[a,b]
end
module Kernel
  alias original_require require
  private :original_require
  alias original_require_relative require_relative
  private :original_require_relative
  alias original_autoload autoload
  alias original_load load
  private :original_load
  # TODO: private opt
  def load(file, priv=false)
    find_path = file
    script = file_path = nil
    FS_LOAD_PATHS.each do |load_path|
      file_path = File.join(load_path, find_path)
      break if (script = Fs.get_file_from_fs(file_path))
    end
    eval_or_require_extension(script, file_path, file, force: true)
  rescue LoadError => e
    find_path = file
    # puts "load local #{find_path}"
    original_load(find_path)
  rescue SyntaxError => e
    puts e.message
  end
  def require(file)
    find_path = file
    script = file_path = nil
    if File.absolute_path?(find_path)
      if File.extname(find_path) == ''
        EXTS.each do |ext|
          file_path = find_path + ext
          break if (script = Fs.get_file_from_fs(file_path))
        end
      else
        file_path = find_path
        script = Fs.get_file_from_fs(file_path)
      end
    else
      if File.extname(file) == ''
        FS_LOAD_PATHS.each do |load_path|
          EXTS.each do |ext|
            find_path = file + ext
            file_path = File.join(load_path, find_path)
            break if (script = Fs.get_file_from_fs(file_path))
          end
          break if script
        end
      else
        FS_LOAD_PATHS.each do |load_path|
          file_path = File.join(load_path, find_path)
          break if (script = Fs.get_file_from_fs(file_path))
        end
      end
    end
    eval_or_require_extension(script, file_path, file)
  rescue LoadError => e
    # if File.extname(find_path) == '.so'
    #   puts "require static linked extension #{find_path}"
    # else
    #   puts "require local #{find_path}"
    # end
    find_path = file
    original_require(find_path)
  rescue SyntaxError => e
    puts e.message
  end
  def require_relative(file)
    find_path = file
    script = file_path = nil
    call_dir = File.dirname(original_caller_locations(1, 1).first.absolute_path)
    if File.extname(file) == ''
      EXTS.each do |ext|
        find_path = file + ext
        file_path = File.expand_path(File.join(call_dir, find_path))
        break if (script = Fs.get_file_from_fs(file_path))
      end
    else
      file_path = File.expand_path(File.join(call_dir, find_path))
      script = Fs.get_file_from_fs(file_path)
    end
    eval_or_require_extension(script, file_path, file)
  rescue LoadError => e
    find_path = file
    file_path = File.expand_path(File.join(call_dir, find_path))
    # puts "require_relative local #{file_path}"
    original_require_relative(file_path)
  rescue SyntaxError => e
    puts e.message
  end
  def eval_or_require_extension(script, file_path, file, force: false)
    if script.nil?
      raise LoadError, "cannot load such file -- #{file}"
    else
      if map = AUTOLOAD_MAP[file]
        return nil if map[2]
        # p "autoload start #{map}"
        map[0].send(:remove_const, map[1]) # HACK: remove const that initialized by Qundef
        RubyVM::InstructionSequence.compile(script, file_path, file_path).eval
        map[2] = true
        return map[0].const_get(map[1])
      end
      if !force && $LOADED_FEATURES.include?(file_path)
        # puts "already loaded: #{file_path}"
        return false
      end
      $LOADED_FEATURES << file_path
      $LOADED_FEATURES.uniq!
      if File.extname(file_path) == '.rb'
        # puts "eval file path: #{file_path}"
        RubyVM::InstructionSequence.compile(script, file_path, file_path).eval
        return true
      else
        # puts "require native extension #{File.basename(file_path)}"
        original_require(File.basename(file_path))
      end
    end
  end
end
module PATCH
  refine File do
    def File.read(file)
      Fs.get_file_from_fs(file)
    end
  end
end
class Fs
  using PATCH
  def self.pack_context
    yield FS_LOAD_PATHS[0]
  end
end
Fs.pack_context do |_context|
  require 'rubygems'
end
RubyVM::InstructionSequence.compile(Fs.get_start_file_script, Fs.get_start_file_name, Fs.get_start_file_name).eval
