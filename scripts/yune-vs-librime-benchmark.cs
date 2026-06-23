using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.Globalization;
using System.IO;
using System.Runtime.InteropServices;
using System.Text;

public static class YuneVsLibrimeBenchmark {
  [StructLayout(LayoutKind.Sequential)]
  public struct RimeTraits {
    public int data_size;
    public IntPtr shared_data_dir;
    public IntPtr user_data_dir;
    public IntPtr distribution_name;
    public IntPtr distribution_code_name;
    public IntPtr distribution_version;
    public IntPtr app_name;
    public IntPtr modules;
    public int min_log_level;
    public IntPtr log_dir;
    public IntPtr prebuilt_data_dir;
    public IntPtr staging_dir;
  }

  [StructLayout(LayoutKind.Sequential)]
  public struct RimeComposition {
    public int length;
    public int cursor_pos;
    public int sel_start;
    public int sel_end;
    public IntPtr preedit;
  }

  [StructLayout(LayoutKind.Sequential)]
  public struct RimeCandidate {
    public IntPtr text;
    public IntPtr comment;
    public IntPtr reserved;
  }

  [StructLayout(LayoutKind.Sequential)]
  public struct RimeMenu {
    public int page_size;
    public int page_no;
    public int is_last_page;
    public int highlighted_candidate_index;
    public int num_candidates;
    public IntPtr candidates;
    public IntPtr select_keys;
  }

  [StructLayout(LayoutKind.Sequential)]
  public struct RimeContext {
    public int data_size;
    public RimeComposition composition;
    public RimeMenu menu;
    public IntPtr commit_text_preview;
    public IntPtr select_labels;
  }

  [StructLayout(LayoutKind.Sequential)]
  public struct RimeStatus {
    public int data_size;
    public IntPtr schema_id;
    public IntPtr schema_name;
    public int is_disabled;
    public int is_composing;
    public int is_ascii_mode;
    public int is_full_shape;
    public int is_simplified;
    public int is_traditional;
    public int is_ascii_punct;
  }

  [StructLayout(LayoutKind.Sequential)]
  public struct ProcessMemoryCounters {
    public uint cb;
    public uint page_fault_count;
    public UIntPtr peak_working_set_size;
    public UIntPtr working_set_size;
    public UIntPtr quota_peak_paged_pool_usage;
    public UIntPtr quota_paged_pool_usage;
    public UIntPtr quota_peak_non_paged_pool_usage;
    public UIntPtr quota_non_paged_pool_usage;
    public UIntPtr pagefile_usage;
    public UIntPtr peak_pagefile_usage;
  }

  public struct MemorySample {
    public ulong workingSetBytes;
    public ulong peakWorkingSetBytes;
  }

  public sealed class Sample {
    public string engine;
    public string workload;
    public string input;
    public int sampleIndex;
    public int operationCount;
    public double totalUs;
    public double usPerOperation;
    public ulong beforeWorkingSetBytes;
    public ulong afterReadyWorkingSetBytes;
    public ulong afterFinalizeWorkingSetBytes;
    public ulong peakWorkingSetBytes;
  }

  [DllImport("rime.dll", CallingConvention = CallingConvention.Cdecl)]
  public static extern void RimeSetup(ref RimeTraits traits);
  [DllImport("rime.dll", CallingConvention = CallingConvention.Cdecl)]
  public static extern void RimeInitialize(ref RimeTraits traits);
  [DllImport("rime.dll", CallingConvention = CallingConvention.Cdecl)]
  public static extern void RimeFinalize();
  [DllImport("rime.dll", CallingConvention = CallingConvention.Cdecl)]
  public static extern UIntPtr RimeCreateSession();
  [DllImport("rime.dll", CallingConvention = CallingConvention.Cdecl)]
  public static extern int RimeDestroySession(UIntPtr session);
  [DllImport("rime.dll", CallingConvention = CallingConvention.Cdecl)]
  public static extern int RimeSelectSchema(UIntPtr session, IntPtr schemaId);
  [DllImport("rime.dll", CallingConvention = CallingConvention.Cdecl)]
  public static extern void RimeSetOption(UIntPtr session, IntPtr option, int value);
  [DllImport("rime.dll", CallingConvention = CallingConvention.Cdecl)]
  public static extern int RimeProcessKey(UIntPtr session, int keycode, int mask);
  [DllImport("rime.dll", CallingConvention = CallingConvention.Cdecl)]
  public static extern int RimeGetContext(UIntPtr session, ref RimeContext context);
  [DllImport("rime.dll", CallingConvention = CallingConvention.Cdecl)]
  public static extern int RimeFreeContext(ref RimeContext context);
  [DllImport("rime.dll", CallingConvention = CallingConvention.Cdecl)]
  public static extern int RimeGetStatus(UIntPtr session, ref RimeStatus status);
  [DllImport("rime.dll", CallingConvention = CallingConvention.Cdecl)]
  public static extern int RimeFreeStatus(ref RimeStatus status);
  [DllImport("rime.dll", CallingConvention = CallingConvention.Cdecl)]
  public static extern void RimeClearComposition(UIntPtr session);

  [DllImport("kernel32.dll")]
  static extern IntPtr GetCurrentProcess();
  [DllImport("psapi.dll", SetLastError = true)]
  static extern bool GetProcessMemoryInfo(
      IntPtr process,
      out ProcessMemoryCounters counters,
      uint size);

  public static int Main(string[] args) {
    try {
      var options = ParseArgs(args);
      string engine = Required(options, "--engine");
      string shared = Path.GetFullPath(Required(options, "--shared"));
      string user = Path.GetFullPath(Required(options, "--user"));
      string build = Path.GetFullPath(Required(options, "--build"));
      string output = Path.GetFullPath(Required(options, "--output"));
      string schema = Option(options, "--schema", "luna_pinyin");
      int iterations = int.Parse(Option(options, "--iterations", "9"), CultureInfo.InvariantCulture);
      int sessionIterations = int.Parse(Option(options, "--session-iterations", "60"), CultureInfo.InvariantCulture);
      int keyIterations = int.Parse(Option(options, "--key-iterations", "80"), CultureInfo.InvariantCulture);

      Directory.CreateDirectory(output);
      var samples = new List<Sample>();
      RunStartup(engine, schema, shared, user, build, iterations, samples);
      RunSession(engine, schema, shared, user, build, sessionIterations, samples);
      RunKeyWorkload(engine, schema, shared, user, build, "ni", keyIterations, samples);
      RunKeyWorkload(engine, schema, shared, user, build, "hao", keyIterations, samples);
      RunKeyWorkload(engine, schema, shared, user, build, "zhongguo", keyIterations, samples);
      WriteSamples(Path.Combine(output, "samples.csv"), samples);
      WriteSummary(Path.Combine(output, "summary.csv"), samples);
      WriteMetadata(Path.Combine(output, "metadata.txt"), engine, schema, shared, user, build, iterations, sessionIterations, keyIterations);
      Console.WriteLine("engine={0}", engine);
      Console.WriteLine("samples={0}", samples.Count);
      Console.WriteLine("summary={0}", Path.Combine(output, "summary.csv"));
      return 0;
    } catch (Exception ex) {
      Console.Error.WriteLine(ex.ToString());
      return 1;
    }
  }

  static void RunStartup(
      string engine,
      string schema,
      string shared,
      string user,
      string build,
      int iterations,
      List<Sample> samples) {
    for (int i = 0; i < iterations; i++) {
      var ptrs = new List<IntPtr>();
      UIntPtr session = UIntPtr.Zero;
      var traits = Traits(shared, user, build, engine, ptrs);
      var before = CurrentMemorySample();
      var sw = Stopwatch.StartNew();
      try {
        RimeSetup(ref traits);
        RimeInitialize(ref traits);
        session = RimeCreateSession();
        if (session == UIntPtr.Zero) {
          throw new Exception("RimeCreateSession returned zero");
        }
        if (RimeSelectSchema(session, U8(schema, ptrs)) == 0) {
          throw new Exception("RimeSelectSchema failed: " + schema);
        }
        ReadStatus(session);
        var ready = CurrentMemorySample();
        sw.Stop();
        RimeDestroySession(session);
        session = UIntPtr.Zero;
        RimeFinalize();
        var finalized = CurrentMemorySample();
        samples.Add(new Sample {
          engine = engine,
          workload = "startup_warm_shared_assets_runtime_ready",
          input = "",
          sampleIndex = i,
          operationCount = 1,
          totalUs = ElapsedUs(sw.Elapsed),
          usPerOperation = ElapsedUs(sw.Elapsed),
          beforeWorkingSetBytes = before.workingSetBytes,
          afterReadyWorkingSetBytes = ready.workingSetBytes,
          afterFinalizeWorkingSetBytes = finalized.workingSetBytes,
          peakWorkingSetBytes = ready.peakWorkingSetBytes
        });
      } finally {
        if (session != UIntPtr.Zero) {
          RimeDestroySession(session);
        }
        try {
          RimeFinalize();
        } catch {
        }
        FreeAll(ptrs);
      }
    }
  }

  static void RunSession(
      string engine,
      string schema,
      string shared,
      string user,
      string build,
      int iterations,
      List<Sample> samples) {
    WithService(engine, shared, user, build, delegate(List<IntPtr> ptrs) {
      for (int i = 0; i < iterations; i++) {
        var before = CurrentMemorySample();
        var sw = Stopwatch.StartNew();
        UIntPtr session = RimeCreateSession();
        if (session == UIntPtr.Zero) {
          throw new Exception("RimeCreateSession returned zero");
        }
        if (RimeSelectSchema(session, U8(schema, ptrs)) == 0) {
          throw new Exception("RimeSelectSchema failed: " + schema);
        }
        if (RimeDestroySession(session) == 0) {
          throw new Exception("RimeDestroySession failed");
        }
        sw.Stop();
        var after = CurrentMemorySample();
        samples.Add(new Sample {
          engine = engine,
          workload = "session_create_select_destroy",
          input = "",
          sampleIndex = i,
          operationCount = 1,
          totalUs = ElapsedUs(sw.Elapsed),
          usPerOperation = ElapsedUs(sw.Elapsed),
          beforeWorkingSetBytes = before.workingSetBytes,
          afterReadyWorkingSetBytes = after.workingSetBytes,
          afterFinalizeWorkingSetBytes = 0,
          peakWorkingSetBytes = after.peakWorkingSetBytes
        });
      }
    });
  }

  static void RunKeyWorkload(
      string engine,
      string schema,
      string shared,
      string user,
      string build,
      string input,
      int iterations,
      List<Sample> samples) {
    WithService(engine, shared, user, build, delegate(List<IntPtr> ptrs) {
      UIntPtr session = RimeCreateSession();
      if (session == UIntPtr.Zero) {
        throw new Exception("RimeCreateSession returned zero");
      }
      try {
        if (RimeSelectSchema(session, U8(schema, ptrs)) == 0) {
          throw new Exception("RimeSelectSchema failed: " + schema);
        }
        RimeSetOption(session, U8("ascii_mode", ptrs), 0);
        RimeSetOption(session, U8("full_shape", ptrs), 0);
        RimeSetOption(session, U8("ascii_punct", ptrs), 0);
        RimeSetOption(session, U8("zh_hans", ptrs), 0);
        for (int warmup = 0; warmup < 5; warmup++) {
          ProcessInputWithContext(session, input);
        }
        for (int i = 0; i < iterations; i++) {
          var before = CurrentMemorySample();
          var sw = Stopwatch.StartNew();
          ProcessInputWithContext(session, input);
          sw.Stop();
          var after = CurrentMemorySample();
          double totalUs = ElapsedUs(sw.Elapsed);
          samples.Add(new Sample {
            engine = engine,
            workload = "key_sequence_process_with_context",
            input = input,
            sampleIndex = i,
            operationCount = input.Length,
            totalUs = totalUs,
            usPerOperation = totalUs / input.Length,
            beforeWorkingSetBytes = before.workingSetBytes,
            afterReadyWorkingSetBytes = after.workingSetBytes,
            afterFinalizeWorkingSetBytes = 0,
            peakWorkingSetBytes = after.peakWorkingSetBytes
          });
        }
      } finally {
        RimeDestroySession(session);
      }
    });
  }

  static void ProcessInputWithContext(UIntPtr session, string input) {
    RimeClearComposition(session);
    foreach (char ch in input) {
      if (RimeProcessKey(session, (int)ch, 0) == 0) {
        throw new Exception("RimeProcessKey failed for " + input);
      }
    }
    ReadContext(session);
  }

  static void WithService(
      string engine,
      string shared,
      string user,
      string build,
      Action<List<IntPtr>> action) {
    var ptrs = new List<IntPtr>();
    var traits = Traits(shared, user, build, engine, ptrs);
    try {
      RimeSetup(ref traits);
      RimeInitialize(ref traits);
      action(ptrs);
    } finally {
      try {
        RimeFinalize();
      } catch {
      }
      FreeAll(ptrs);
    }
  }

  static RimeTraits Traits(string shared, string user, string build, string engine, List<IntPtr> ptrs) {
    var modules = new List<IntPtr>();
    modules.Add(U8("default", ptrs));
    return new RimeTraits {
      data_size = Marshal.SizeOf(typeof(RimeTraits)) - sizeof(int),
      shared_data_dir = U8(shared, ptrs),
      user_data_dir = U8(user, ptrs),
      distribution_name = U8(engine, ptrs),
      distribution_code_name = U8(engine, ptrs),
      distribution_version = U8("benchmark", ptrs),
      app_name = U8("yune.vs.librime.benchmark", ptrs),
      modules = U8Array(modules, ptrs),
      min_log_level = 2,
      log_dir = U8("", ptrs),
      prebuilt_data_dir = U8(build, ptrs),
      staging_dir = U8(build, ptrs)
    };
  }

  static void ReadContext(UIntPtr session) {
    var context = new RimeContext {
      data_size = Marshal.SizeOf(typeof(RimeContext)) - sizeof(int)
    };
    if (RimeGetContext(session, ref context) == 0) {
      throw new Exception("RimeGetContext failed");
    }
    RimeFreeContext(ref context);
  }

  static void ReadStatus(UIntPtr session) {
    var status = new RimeStatus {
      data_size = Marshal.SizeOf(typeof(RimeStatus)) - sizeof(int)
    };
    if (RimeGetStatus(session, ref status) == 0) {
      throw new Exception("RimeGetStatus failed");
    }
    RimeFreeStatus(ref status);
  }

  static MemorySample CurrentMemorySample() {
    ProcessMemoryCounters counters;
    counters.cb = (uint)Marshal.SizeOf(typeof(ProcessMemoryCounters));
    if (!GetProcessMemoryInfo(
        GetCurrentProcess(),
        out counters,
        (uint)Marshal.SizeOf(typeof(ProcessMemoryCounters)))) {
      return new MemorySample();
    }
    return new MemorySample {
      workingSetBytes = counters.working_set_size.ToUInt64(),
      peakWorkingSetBytes = counters.peak_working_set_size.ToUInt64()
    };
  }

  static void WriteSamples(string path, List<Sample> samples) {
    var lines = new List<string>();
    lines.Add("engine,workload,input,sample_index,operation_count,total_us,us_per_operation,before_working_set_bytes,after_ready_working_set_bytes,after_finalize_working_set_bytes,peak_working_set_bytes");
    foreach (var sample in samples) {
      lines.Add(string.Join(",", new[] {
        Csv(sample.engine),
        Csv(sample.workload),
        Csv(sample.input),
        sample.sampleIndex.ToString(CultureInfo.InvariantCulture),
        sample.operationCount.ToString(CultureInfo.InvariantCulture),
        F(sample.totalUs),
        F(sample.usPerOperation),
        sample.beforeWorkingSetBytes.ToString(CultureInfo.InvariantCulture),
        sample.afterReadyWorkingSetBytes.ToString(CultureInfo.InvariantCulture),
        sample.afterFinalizeWorkingSetBytes.ToString(CultureInfo.InvariantCulture),
        sample.peakWorkingSetBytes.ToString(CultureInfo.InvariantCulture)
      }));
    }
    File.WriteAllLines(path, lines, new UTF8Encoding(false));
  }

  static void WriteSummary(string path, List<Sample> samples) {
    var groups = new SortedDictionary<string, List<Sample>>();
    foreach (var sample in samples) {
      string key = sample.engine + "|" + sample.workload + "|" + sample.input;
      if (!groups.ContainsKey(key)) {
        groups[key] = new List<Sample>();
      }
      groups[key].Add(sample);
    }

    var lines = new List<string>();
    lines.Add("engine,workload,input,iterations,operation_count_per_sample,median_us,p95_us,min_us,max_us,median_us_per_operation,p95_us_per_operation,median_after_ready_working_set_bytes,median_ready_delta_bytes,median_peak_working_set_bytes");
    foreach (var group in groups.Values) {
      group.Sort((a, b) => a.totalUs.CompareTo(b.totalUs));
      var us = new List<double>();
      var usPerOp = new List<double>();
      var afterReady = new List<ulong>();
      var readyDelta = new List<ulong>();
      var peak = new List<ulong>();
      foreach (var sample in group) {
        us.Add(sample.totalUs);
        usPerOp.Add(sample.usPerOperation);
        afterReady.Add(sample.afterReadyWorkingSetBytes);
        if (sample.afterReadyWorkingSetBytes >= sample.beforeWorkingSetBytes) {
          readyDelta.Add(sample.afterReadyWorkingSetBytes - sample.beforeWorkingSetBytes);
        }
        peak.Add(sample.peakWorkingSetBytes);
      }
      var first = group[0];
      lines.Add(string.Join(",", new[] {
        Csv(first.engine),
        Csv(first.workload),
        Csv(first.input),
        group.Count.ToString(CultureInfo.InvariantCulture),
        first.operationCount.ToString(CultureInfo.InvariantCulture),
        F(Percentile(us, 0.50)),
        F(Percentile(us, 0.95)),
        F(us[0]),
        F(us[us.Count - 1]),
        F(Percentile(usPerOp, 0.50)),
        F(Percentile(usPerOp, 0.95)),
        MedianUlong(afterReady).ToString(CultureInfo.InvariantCulture),
        MedianUlong(readyDelta).ToString(CultureInfo.InvariantCulture),
        MedianUlong(peak).ToString(CultureInfo.InvariantCulture)
      }));
    }
    File.WriteAllLines(path, lines, new UTF8Encoding(false));
  }

  static void WriteMetadata(
      string path,
      string engine,
      string schema,
      string shared,
      string user,
      string build,
      int startupIterations,
      int sessionIterations,
      int keyIterations) {
    var lines = new List<string>();
    lines.Add("engine=" + engine);
    lines.Add("schema=" + schema);
    lines.Add("shared=" + shared);
    lines.Add("user=" + user);
    lines.Add("build=" + build);
    lines.Add("startup_iterations=" + startupIterations.ToString(CultureInfo.InvariantCulture));
    lines.Add("session_iterations=" + sessionIterations.ToString(CultureInfo.InvariantCulture));
    lines.Add("key_iterations=" + keyIterations.ToString(CultureInfo.InvariantCulture));
    lines.Add("runner_process=" + Process.GetCurrentProcess().MainModule.FileName);
    lines.Add("timestamp_utc=" + DateTime.UtcNow.ToString("o", CultureInfo.InvariantCulture));
    File.WriteAllLines(path, lines, new UTF8Encoding(false));
  }

  static Dictionary<string, string> ParseArgs(string[] args) {
    var parsed = new Dictionary<string, string>(StringComparer.OrdinalIgnoreCase);
    for (int i = 0; i < args.Length; i += 2) {
      if (i + 1 >= args.Length) {
        throw new ArgumentException("missing value for " + args[i]);
      }
      parsed[args[i]] = args[i + 1];
    }
    return parsed;
  }

  static string Required(Dictionary<string, string> options, string name) {
    string value;
    if (!options.TryGetValue(name, out value) || string.IsNullOrWhiteSpace(value)) {
      throw new ArgumentException("missing required option " + name);
    }
    return value;
  }

  static string Option(Dictionary<string, string> options, string name, string fallback) {
    string value;
    return options.TryGetValue(name, out value) ? value : fallback;
  }

  static IntPtr U8(string value, List<IntPtr> ptrs) {
    byte[] bytes = Encoding.UTF8.GetBytes(value ?? "");
    IntPtr p = Marshal.AllocHGlobal(bytes.Length + 1);
    Marshal.Copy(bytes, 0, p, bytes.Length);
    Marshal.WriteByte(p, bytes.Length, 0);
    ptrs.Add(p);
    return p;
  }

  static IntPtr U8Array(List<IntPtr> values, List<IntPtr> ptrs) {
    IntPtr array = Marshal.AllocHGlobal(IntPtr.Size * (values.Count + 1));
    for (int i = 0; i < values.Count; i++) {
      Marshal.WriteIntPtr(array, i * IntPtr.Size, values[i]);
    }
    Marshal.WriteIntPtr(array, values.Count * IntPtr.Size, IntPtr.Zero);
    ptrs.Add(array);
    return array;
  }

  static void FreeAll(List<IntPtr> ptrs) {
    foreach (IntPtr ptr in ptrs) {
      Marshal.FreeHGlobal(ptr);
    }
  }

  static double ElapsedUs(TimeSpan elapsed) {
    return elapsed.TotalMilliseconds * 1000.0;
  }

  static string F(double value) {
    return value.ToString("0.000", CultureInfo.InvariantCulture);
  }

  static double Percentile(List<double> sortedValues, double percentile) {
    sortedValues.Sort();
    if (sortedValues.Count == 0) {
      return 0.0;
    }
    int index = (int)Math.Ceiling(percentile * sortedValues.Count) - 1;
    if (index < 0) {
      index = 0;
    }
    if (index >= sortedValues.Count) {
      index = sortedValues.Count - 1;
    }
    return sortedValues[index];
  }

  static ulong MedianUlong(List<ulong> values) {
    if (values.Count == 0) {
      return 0;
    }
    values.Sort();
    return values[values.Count / 2];
  }

  static string Csv(string value) {
    value = value ?? "";
    if (value.IndexOfAny(new[] { ',', '"', '\n', '\r' }) < 0) {
      return value;
    }
    return "\"" + value.Replace("\"", "\"\"") + "\"";
  }
}
