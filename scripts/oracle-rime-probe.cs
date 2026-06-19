using System;
using System.Collections.Generic;
using System.IO;
using System.Runtime.InteropServices;
using System.Text;

public static class RimeProbe {
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
  public struct RimeCommit {
    public int data_size;
    public IntPtr text;
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

  public class ProbeAction {
    public string type;
    public string text;
    public int keycode;
    public int mask;
    public string option;
    public int value;
    public string label;
  }

  public class ProbeScenario {
    public string name;
    public ProbeAction[] actions;
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
  public static extern int RimeGetCommit(UIntPtr session, ref RimeCommit commit);
  [DllImport("rime.dll", CallingConvention = CallingConvention.Cdecl)]
  public static extern int RimeFreeCommit(ref RimeCommit commit);
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

  static IntPtr U8(string value, List<IntPtr> ptrs) {
    byte[] bytes = Encoding.UTF8.GetBytes(value);
    IntPtr p = Marshal.AllocHGlobal(bytes.Length + 1);
    Marshal.Copy(bytes, 0, p, bytes.Length);
    Marshal.WriteByte(p, bytes.Length, 0);
    ptrs.Add(p);
    return p;
  }

  static string S(IntPtr value) {
    if (value == IntPtr.Zero) {
      return null;
    }
    int len = 0;
    while (Marshal.ReadByte(value, len) != 0) {
      len++;
    }
    byte[] bytes = new byte[len];
    Marshal.Copy(value, bytes, 0, len);
    return Encoding.UTF8.GetString(bytes);
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

  static RimeTraits Traits(
      string shared,
      string user,
      string build,
      string[] modulesInput,
      List<IntPtr> ptrs) {
    var modules = new List<IntPtr>();
    foreach (var module in modulesInput) {
      modules.Add(U8(module, ptrs));
    }
    return new RimeTraits {
      data_size = Marshal.SizeOf(typeof(RimeTraits)) - sizeof(int),
      shared_data_dir = U8(shared, ptrs),
      user_data_dir = U8(user, ptrs),
      distribution_name = U8("Rime", ptrs),
      distribution_code_name = U8("rime", ptrs),
      distribution_version = U8("1.17.0", ptrs),
      app_name = U8("rime.yune_upstream_oracle_probe", ptrs),
      modules = U8Array(modules, ptrs),
      min_log_level = 2,
      log_dir = U8("", ptrs),
      prebuilt_data_dir = U8(build, ptrs),
      staging_dir = U8(build, ptrs),
    };
  }

  static string SanitizePathSegment(string value) {
    if (string.IsNullOrEmpty(value)) {
      return "unnamed";
    }
    var builder = new StringBuilder(value.Length);
    foreach (var ch in value) {
      if (char.IsLetterOrDigit(ch) || ch == '-' || ch == '_') {
        builder.Append(ch);
      } else {
        builder.Append('_');
      }
    }
    return builder.Length == 0 ? "unnamed" : builder.ToString();
  }

  static string TakeCommit(UIntPtr session) {
    var commit = new RimeCommit { data_size = Marshal.SizeOf(typeof(RimeCommit)) - sizeof(int) };
    if (RimeGetCommit(session, ref commit) == 0) {
      return null;
    }
    var text = S(commit.text);
    RimeFreeCommit(ref commit);
    return text;
  }

  static Dictionary<string, object> Snapshot(
      UIntPtr session,
      string scenario,
      string label,
      string commitText) {
    var ctx = new RimeContext { data_size = Marshal.SizeOf(typeof(RimeContext)) - sizeof(int) };
    var status = new RimeStatus { data_size = Marshal.SizeOf(typeof(RimeStatus)) - sizeof(int) };
    if (RimeGetContext(session, ref ctx) == 0) {
      throw new Exception("RimeGetContext failed for scenario " + scenario + " snapshot " + label);
    }
    if (RimeGetStatus(session, ref status) == 0) {
      RimeFreeContext(ref ctx);
      throw new Exception("RimeGetStatus failed for scenario " + scenario + " snapshot " + label);
    }

    var candidates = new List<Dictionary<string, object>>();
    int candSize = Marshal.SizeOf(typeof(RimeCandidate));
    for (int i = 0; i < ctx.menu.num_candidates; i++) {
      var cand = (RimeCandidate)Marshal.PtrToStructure(
          IntPtr.Add(ctx.menu.candidates, i * candSize),
          typeof(RimeCandidate));
      var row = new Dictionary<string, object>();
      row["index"] = i;
      row["text"] = S(cand.text);
      row["comment"] = S(cand.comment);
      candidates.Add(row);
    }

    var result = new Dictionary<string, object>();
    result["schema_id"] = S(status.schema_id);
    result["schema_name"] = S(status.schema_name);
    result["scenario"] = scenario;
    result["label"] = label;
    result["is_composing"] = status.is_composing != 0;
    result["is_ascii_mode"] = status.is_ascii_mode != 0;
    result["is_full_shape"] = status.is_full_shape != 0;
    result["is_simplified"] = status.is_simplified != 0;
    result["is_traditional"] = status.is_traditional != 0;
    result["is_ascii_punct"] = status.is_ascii_punct != 0;
    result["composition_length"] = ctx.composition.length;
    result["cursor_pos"] = ctx.composition.cursor_pos;
    result["preedit"] = S(ctx.composition.preedit);
    result["commit_text_preview"] = S(ctx.commit_text_preview);
    result["commit_text"] = commitText;
    result["highlighted_candidate_index"] = ctx.menu.highlighted_candidate_index;
    result["page_size"] = ctx.menu.page_size;
    result["page_no"] = ctx.menu.page_no;
    result["is_last_page"] = ctx.menu.is_last_page != 0;
    result["selected_candidates"] = candidates;
    RimeFreeStatus(ref status);
    RimeFreeContext(ref ctx);
    return result;
  }

  public static List<Dictionary<string, object>> Capture(
      string shared,
      string user,
      string build,
      string schema,
      string[] modulesInput,
      string[] inputs) {
    var ptrs = new List<IntPtr>();
    var traits = Traits(shared, user, build, modulesInput, ptrs);
    var results = new List<Dictionary<string, object>>();
    UIntPtr session = UIntPtr.Zero;
    try {
      RimeSetup(ref traits);
      RimeInitialize(ref traits);
      session = RimeCreateSession();
      if (session == UIntPtr.Zero) {
        throw new Exception("RimeCreateSession returned zero");
      }
      var schemaPtr = U8(schema, ptrs);
      if (RimeSelectSchema(session, schemaPtr) == 0) {
        throw new Exception("RimeSelectSchema failed: " + schema);
      }
      RimeSetOption(session, U8("ascii_mode", ptrs), 0);

      foreach (var input in inputs) {
        RimeClearComposition(session);
        var processed = new List<int>();
        foreach (var ch in input) {
          processed.Add(RimeProcessKey(session, (int)ch, 0));
        }
        var ctx = new RimeContext { data_size = Marshal.SizeOf(typeof(RimeContext)) - sizeof(int) };
        var status = new RimeStatus { data_size = Marshal.SizeOf(typeof(RimeStatus)) - sizeof(int) };
        if (RimeGetContext(session, ref ctx) == 0) {
          throw new Exception("RimeGetContext failed for " + input);
        }
        if (RimeGetStatus(session, ref status) == 0) {
          throw new Exception("RimeGetStatus failed for " + input);
        }

        var candidates = new List<Dictionary<string, object>>();
        int candSize = Marshal.SizeOf(typeof(RimeCandidate));
        for (int i = 0; i < ctx.menu.num_candidates; i++) {
          var cand = (RimeCandidate)Marshal.PtrToStructure(
              IntPtr.Add(ctx.menu.candidates, i * candSize),
              typeof(RimeCandidate));
          var row = new Dictionary<string, object>();
          row["index"] = i;
          row["text"] = S(cand.text);
          row["comment"] = S(cand.comment);
          candidates.Add(row);
        }

        var result = new Dictionary<string, object>();
        result["schema_id"] = S(status.schema_id);
        result["schema_name"] = S(status.schema_name);
        result["input"] = input;
        result["processed"] = processed;
        result["is_composing"] = status.is_composing != 0;
        result["is_ascii_mode"] = status.is_ascii_mode != 0;
        result["preedit"] = S(ctx.composition.preedit);
        result["commit_text_preview"] = S(ctx.commit_text_preview);
        result["highlighted_candidate_index"] = ctx.menu.highlighted_candidate_index;
        result["page_size"] = ctx.menu.page_size;
        result["page_no"] = ctx.menu.page_no;
        result["is_last_page"] = ctx.menu.is_last_page != 0;
        result["selected_candidates"] = candidates;
        results.Add(result);
        RimeFreeStatus(ref status);
        RimeFreeContext(ref ctx);
      }
      RimeDestroySession(session);
      session = UIntPtr.Zero;
      return results;
    } finally {
      if (session != UIntPtr.Zero) {
        RimeDestroySession(session);
      }
      RimeFinalize();
      foreach (var p in ptrs) {
        Marshal.FreeHGlobal(p);
      }
    }
  }

  public static List<Dictionary<string, object>> CaptureScenarios(
      string shared,
      string user,
      string build,
      string schema,
      string[] modulesInput,
      ProbeScenario[] scenarios) {
    var results = new List<Dictionary<string, object>>();
    var scenarioRoot = Path.Combine(user, "scenarios");
    if (Directory.Exists(scenarioRoot)) {
      Directory.Delete(scenarioRoot, true);
    }
    Directory.CreateDirectory(scenarioRoot);

    foreach (var scenario in scenarios) {
      var ptrs = new List<IntPtr>();
      var scenarioUser = Path.Combine(scenarioRoot, SanitizePathSegment(scenario.name));
      Directory.CreateDirectory(scenarioUser);
      var traits = Traits(shared, scenarioUser, build, modulesInput, ptrs);
      UIntPtr session = UIntPtr.Zero;
      RimeSetup(ref traits);
      RimeInitialize(ref traits);

      try {
        session = RimeCreateSession();
        if (session == UIntPtr.Zero) {
          throw new Exception("RimeCreateSession returned zero");
        }
        var schemaPtr = U8(schema, ptrs);
        if (RimeSelectSchema(session, schemaPtr) == 0) {
          throw new Exception("RimeSelectSchema failed: " + schema);
        }
        RimeSetOption(session, U8("ascii_mode", ptrs), 0);
        RimeSetOption(session, U8("full_shape", ptrs), 0);
        RimeSetOption(session, U8("ascii_punct", ptrs), 0);
        RimeSetOption(session, U8("zh_hans", ptrs), 0);

        foreach (var action in scenario.actions ?? Array.Empty<ProbeAction>()) {
          var type = action.type ?? "";
          if (type == "input") {
            foreach (var ch in action.text ?? "") {
              RimeProcessKey(session, (int)ch, 0);
              var commit = TakeCommit(session);
              if (commit != null) {
                var label = string.IsNullOrEmpty(action.label)
                    ? "after_input_commit"
                    : action.label;
                results.Add(Snapshot(session, scenario.name, label, commit));
              }
            }
          } else if (type == "key") {
            RimeProcessKey(session, action.keycode, action.mask);
            var commit = TakeCommit(session);
            if (commit != null) {
              var label = string.IsNullOrEmpty(action.label)
                  ? "after_key_" + action.keycode.ToString()
                  : action.label;
              results.Add(Snapshot(session, scenario.name, label, commit));
            }
          } else if (type == "set_option") {
            RimeSetOption(session, U8(action.option ?? "", ptrs), action.value);
          } else if (type == "clear") {
            RimeClearComposition(session);
          } else if (type == "snapshot") {
            results.Add(Snapshot(session, scenario.name, action.label ?? "snapshot", null));
          } else {
            throw new Exception("unsupported scenario action type: " + type);
          }
        }

        RimeDestroySession(session);
        session = UIntPtr.Zero;
      } finally {
        if (session != UIntPtr.Zero) {
          RimeDestroySession(session);
        }
        RimeFinalize();
        foreach (var p in ptrs) {
          Marshal.FreeHGlobal(p);
        }
      }
    }
    return results;
  }
}
