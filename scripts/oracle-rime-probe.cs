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
  public struct RimeCandidateWithQuality {
    public IntPtr text;
    public IntPtr comment;
    public double quality;
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
  public struct RimeSchemaListItem {
    public IntPtr schema_id;
    public IntPtr name;
    public IntPtr reserved;
  }

  [StructLayout(LayoutKind.Sequential)]
  public struct RimeSchemaList {
    public UIntPtr size;
    public IntPtr list;
  }

  [StructLayout(LayoutKind.Sequential)]
  public struct RimeStringSlice {
    public IntPtr str;
    public UIntPtr length;
  }

  [StructLayout(LayoutKind.Sequential)]
  public struct RimeModule {
    public int data_size;
    public IntPtr module_name;
    public IntPtr initialize;
    public IntPtr finalize;
    public IntPtr get_api;
  }

  [StructLayout(LayoutKind.Sequential)]
  public struct RimeLeversApi {
    public int data_size;
    public IntPtr custom_settings_init;
    public IntPtr custom_settings_destroy;
    public IntPtr load_settings;
    public IntPtr save_settings;
    public IntPtr customize_bool;
    public IntPtr customize_int;
    public IntPtr customize_double;
    public IntPtr customize_string;
    public IntPtr is_first_run;
    public IntPtr settings_is_modified;
    public IntPtr settings_get_config;
    public IntPtr switcher_settings_init;
    public IntPtr get_available_schema_list;
    public IntPtr get_selected_schema_list;
    public IntPtr schema_list_destroy;
    public IntPtr get_schema_id;
    public IntPtr get_schema_name;
    public IntPtr get_schema_version;
    public IntPtr get_schema_author;
    public IntPtr get_schema_description;
    public IntPtr get_schema_file_path;
    public IntPtr select_schemas;
    public IntPtr get_hotkeys;
    public IntPtr set_hotkeys;
    public IntPtr user_dict_iterator_init;
    public IntPtr user_dict_iterator_destroy;
    public IntPtr next_user_dict;
    public IntPtr backup_user_dict;
    public IntPtr restore_user_dict;
    public IntPtr export_user_dict;
    public IntPtr import_user_dict;
    public IntPtr customize_item;
  }

  [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
  delegate IntPtr GetCustomApiDelegate();

  [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
  delegate int ExportUserDictDelegate(IntPtr dictName, IntPtr textFile);

  [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
  delegate int ImportUserDictDelegate(IntPtr dictName, IntPtr textFile);

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

  public class ProbeIdentity {
    public string distribution_name;
    public string distribution_code_name;
    public string distribution_version;
    public string app_name;
    public bool candidate_has_quality;
  }

  public sealed class RuntimeOptionSetting {
    public readonly string name;
    public readonly bool enabled;

    public RuntimeOptionSetting(string name, bool enabled) {
      this.name = name;
      this.enabled = enabled;
    }
  }

  static readonly RuntimeOptionSetting[] CaptureRuntimeOptionPolicy = new RuntimeOptionSetting[] {
    new RuntimeOptionSetting("ascii_mode", false),
    new RuntimeOptionSetting("full_shape", false),
    new RuntimeOptionSetting("ascii_punct", false),
    new RuntimeOptionSetting("zh_hans", false),
  };

  public static RuntimeOptionSetting[] GetCaptureRuntimeOptions() {
    return (RuntimeOptionSetting[])CaptureRuntimeOptionPolicy.Clone();
  }

  public static string CaptureRuntimeOptionsSource {
    get { return "RimeProbe.CaptureWithIdentity/CaptureRuntimeOptionPolicy"; }
  }

  public static string SharedCaptureRuntimeOptionsSource {
    get {
      return "RimeProbe.CaptureWithIdentity+CaptureScenariosWithIdentity/CaptureRuntimeOptionPolicy";
    }
  }

  [DllImport("rime.dll", CallingConvention = CallingConvention.Cdecl)]
  public static extern void RimeSetup(ref RimeTraits traits);
  [DllImport("rime.dll", CallingConvention = CallingConvention.Cdecl)]
  public static extern void RimeInitialize(ref RimeTraits traits);
  [DllImport("rime.dll", CallingConvention = CallingConvention.Cdecl)]
  public static extern int RimeDeployWorkspace();
  [DllImport("rime.dll", CallingConvention = CallingConvention.Cdecl)]
  public static extern void RimeFinalize();
  [DllImport("rime.dll", CallingConvention = CallingConvention.Cdecl)]
  public static extern UIntPtr RimeCreateSession();
  [DllImport("rime.dll", CallingConvention = CallingConvention.Cdecl)]
  public static extern int RimeDestroySession(UIntPtr session);
  [DllImport("rime.dll", CallingConvention = CallingConvention.Cdecl)]
  public static extern int RimeSelectSchema(UIntPtr session, IntPtr schemaId);
  [DllImport("rime.dll", CallingConvention = CallingConvention.Cdecl)]
  public static extern int RimeSyncUserData();
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
  public static extern int RimeGetSchemaList(ref RimeSchemaList schemaList);
  [DllImport("rime.dll", CallingConvention = CallingConvention.Cdecl)]
  public static extern void RimeFreeSchemaList(ref RimeSchemaList schemaList);
  [DllImport("rime.dll", CallingConvention = CallingConvention.Cdecl)]
  public static extern IntPtr RimeFindModule(IntPtr moduleName);
  [DllImport("rime.dll", CallingConvention = CallingConvention.Cdecl)]
  public static extern int RimeFreeContext(ref RimeContext context);
  [DllImport("rime.dll", CallingConvention = CallingConvention.Cdecl)]
  public static extern int RimeGetStatus(UIntPtr session, ref RimeStatus status);
  [DllImport("rime.dll", CallingConvention = CallingConvention.Cdecl)]
  public static extern int RimeFreeStatus(ref RimeStatus status);
  [DllImport("rime.dll", CallingConvention = CallingConvention.Cdecl, EntryPoint = "rime_get_api")]
  static extern IntPtr RimeGetApi();

  [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
  delegate IntPtr RimeGetInputApiFn(UIntPtr session);
  [DllImport("rime.dll", CallingConvention = CallingConvention.Cdecl)]
  public static extern IntPtr RimeGetStateLabel(UIntPtr session, IntPtr optionName, int state);
  [DllImport("rime.dll", CallingConvention = CallingConvention.Cdecl)]
  public static extern RimeStringSlice RimeGetStateLabelAbbreviated(
      UIntPtr session,
      IntPtr optionName,
      int state,
      int abbreviated);
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

  static string SliceS(RimeStringSlice value) {
    if (value.str == IntPtr.Zero) {
      return null;
    }
    ulong length = value.length.ToUInt64();
    if (length > int.MaxValue) {
      throw new Exception("RimeStringSlice length is too large");
    }
    byte[] bytes = new byte[(int)length];
    Marshal.Copy(value.str, bytes, 0, (int)length);
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

  public static ProbeIdentity UpstreamIdentity() {
    return new ProbeIdentity {
      distribution_name = "Rime",
      distribution_code_name = "rime",
      distribution_version = "1.17.0",
      app_name = "rime.yune_upstream_oracle_probe",
      candidate_has_quality = false,
    };
  }

  public static ProbeIdentity TypeDuckV112Identity() {
    return new ProbeIdentity {
      distribution_name = "TypeDuck",
      distribution_code_name = "TypeDuck",
      distribution_version = "v1.1.2",
      app_name = "rime.yune_typeduck_v112_oracle_probe",
      candidate_has_quality = true,
    };
  }

  static RimeTraits Traits(
      string shared,
      string user,
      string build,
      string[] modulesInput,
      ProbeIdentity identity,
      List<IntPtr> ptrs) {
    var modules = new List<IntPtr>();
    foreach (var module in modulesInput) {
      modules.Add(U8(module, ptrs));
    }
    var resolvedIdentity = identity ?? UpstreamIdentity();
    return new RimeTraits {
      data_size = Marshal.SizeOf(typeof(RimeTraits)) - sizeof(int),
      shared_data_dir = U8(shared, ptrs),
      user_data_dir = U8(user, ptrs),
      distribution_name = U8(resolvedIdentity.distribution_name ?? "Rime", ptrs),
      distribution_code_name = U8(resolvedIdentity.distribution_code_name ?? "rime", ptrs),
      distribution_version = U8(resolvedIdentity.distribution_version ?? "1.17.0", ptrs),
      app_name = U8(resolvedIdentity.app_name ?? "rime.yune_upstream_oracle_probe", ptrs),
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

  static List<Dictionary<string, object>> ReadCandidates(RimeContext ctx, ProbeIdentity identity) {
    var candidates = new List<Dictionary<string, object>>();
    var resolvedIdentity = identity ?? UpstreamIdentity();
    if (resolvedIdentity.candidate_has_quality) {
      int candSize = Marshal.SizeOf(typeof(RimeCandidateWithQuality));
      for (int i = 0; i < ctx.menu.num_candidates; i++) {
        var cand = (RimeCandidateWithQuality)Marshal.PtrToStructure(
            IntPtr.Add(ctx.menu.candidates, i * candSize),
            typeof(RimeCandidateWithQuality));
        var row = new Dictionary<string, object>();
        row["index"] = i;
        row["text"] = S(cand.text);
        row["comment"] = S(cand.comment);
        row["quality"] = cand.quality;
        candidates.Add(row);
      }
    } else {
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
    }
    return candidates;
  }

  static List<Dictionary<string, object>> ReadSchemaList(RimeSchemaList list) {
    var schemas = new List<Dictionary<string, object>>();
    int itemSize = Marshal.SizeOf(typeof(RimeSchemaListItem));
    for (ulong i = 0; i < list.size.ToUInt64(); i++) {
      var item = (RimeSchemaListItem)Marshal.PtrToStructure(
          IntPtr.Add(list.list, checked((int)(i * (ulong)itemSize))),
          typeof(RimeSchemaListItem));
      var row = new Dictionary<string, object>();
      row["index"] = (long)i;
      row["schema_id"] = S(item.schema_id);
      row["name"] = S(item.name);
      schemas.Add(row);
    }
    return schemas;
  }

  static string CurrentInput(UIntPtr session, RimeContext ctx) {
    const int GetInputSlot = 69;
    var api = RimeGetApi();
    if (api == IntPtr.Zero) {
      throw new Exception("rime_get_api returned null while resolving get_input");
    }
    int firstSlotOffset = IntPtr.Size == 8 ? 8 : 4;
    int getInputOffset = firstSlotOffset + GetInputSlot * IntPtr.Size;
    int dataSize = Marshal.ReadInt32(api);
    if (dataSize + sizeof(int) < getInputOffset + IntPtr.Size) {
      throw new Exception("RimeApi is too small to contain get_input slot 69");
    }
    var function = Marshal.ReadIntPtr(api, getInputOffset);
    if (function == IntPtr.Zero) {
      throw new Exception("RimeApi get_input slot 69 is null");
    }
    var getInput = (RimeGetInputApiFn)Marshal.GetDelegateForFunctionPointer(
        function, typeof(RimeGetInputApiFn));
    return S(getInput(session));
  }

  static Dictionary<string, object> Snapshot(
      UIntPtr session,
      string scenario,
      string label,
      string commitText,
      ProbeIdentity identity) {
    var ctx = new RimeContext { data_size = Marshal.SizeOf(typeof(RimeContext)) - sizeof(int) };
    var status = new RimeStatus { data_size = Marshal.SizeOf(typeof(RimeStatus)) - sizeof(int) };
    if (RimeGetContext(session, ref ctx) == 0) {
      throw new Exception("RimeGetContext failed for scenario " + scenario + " snapshot " + label);
    }
    if (RimeGetStatus(session, ref status) == 0) {
      RimeFreeContext(ref ctx);
      throw new Exception("RimeGetStatus failed for scenario " + scenario + " snapshot " + label);
    }

    var candidates = ReadCandidates(ctx, identity);

    var result = new Dictionary<string, object>();
    result["schema_id"] = S(status.schema_id);
    result["schema_name"] = S(status.schema_name);
    result["scenario"] = scenario;
    result["label"] = label;
    result["rime_get_input"] = CurrentInput(session, ctx);
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

  public static int DeployWorkspace(
      string shared,
      string user,
      string build,
      string[] modulesInput) {
    return DeployWorkspaceWithIdentity(shared, user, build, modulesInput, UpstreamIdentity());
  }

  public static int DeployWorkspaceWithIdentity(
      string shared,
      string user,
      string build,
      string[] modulesInput,
      ProbeIdentity identity) {
    var ptrs = new List<IntPtr>();
    var traits = Traits(shared, user, build, modulesInput, identity, ptrs);
    try {
      RimeSetup(ref traits);
      RimeInitialize(ref traits);
      return RimeDeployWorkspace();
    } finally {
      RimeFinalize();
      foreach (var p in ptrs) {
        Marshal.FreeHGlobal(p);
      }
    }
  }

  public static List<Dictionary<string, object>> Capture(
      string shared,
      string user,
      string build,
      string schema,
      string[] modulesInput,
      string[] inputs) {
    return CaptureWithIdentity(shared, user, build, schema, modulesInput, inputs, UpstreamIdentity());
  }

  public static List<Dictionary<string, object>> CaptureWithIdentity(
      string shared,
      string user,
      string build,
      string schema,
      string[] modulesInput,
      string[] inputs,
      ProbeIdentity identity) {
    var ptrs = new List<IntPtr>();
    var traits = Traits(shared, user, build, modulesInput, identity, ptrs);
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
      foreach (var option in CaptureRuntimeOptionPolicy) {
        RimeSetOption(session, U8(option.name, ptrs), option.enabled ? 1 : 0);
      }

      foreach (var input in inputs) {
        RimeClearComposition(session);
        var processed = new List<int>();
        foreach (var ch in input) {
          int handled = RimeProcessKey(session, (int)ch, 0);
          if (handled == 0) {
            throw new Exception(
                "RimeProcessKey did not handle input key " + ((int)ch).ToString() +
                " while capturing " + input);
          }
          processed.Add(handled);
        }
        var status = new RimeStatus { data_size = Marshal.SizeOf(typeof(RimeStatus)) - sizeof(int) };
        if (RimeGetStatus(session, ref status) == 0) {
          throw new Exception("RimeGetStatus failed for " + input);
        }

        // Page through every candidate page so the capture is not truncated to
        // page 0. Page_Down is XK_Page_Down (0xff56). For an oracle capture we
        // must never silently duplicate a page: if Page_Down is not handled or
        // the page does not advance (a repeated page_no), stop and record a hard
        // pagination_error instead of producing a plausible-but-wrong blob.
        // A zeroed initial menu is a complete no-menu result with no page rows;
        // an empty page with a positive page size is incomplete unless terminal.
        // Existing page-0 fields (selected_candidates, page_size, is_last_page)
        // are preserved; all_candidates/pages/global_index are new.
        const int PageDownKeycode = 0xff56;
        const int MaxPages = 2000;
        var pages = new List<Dictionary<string, object>>();
        var allCandidates = new List<Dictionary<string, object>>();
        var seenPageNos = new HashSet<int>();
        List<Dictionary<string, object>> firstPageCandidates = null;
        string preedit = null;
        string commitPreview = null;
        string rimeInput = null;
        int highlighted = 0;
        int firstPageSize = 0;
        int firstPageNo = 0;
        int firstPageNumCandidates = 0;
        bool firstPageIsLast = true;
        bool firstPageCandidatePointerNull = true;
        bool menuPresent = true;
        bool capturedAllPages = false;
        string paginationError = null;
        string terminationReason = null;
        int globalIndex = 0;
        for (int pageIndex = 0; ; pageIndex++) {
          if (pageIndex >= MaxPages) {
            paginationError = "max_pages_reached_" + MaxPages;
            terminationReason = "max_pages";
            break;
          }
          var ctx = new RimeContext { data_size = Marshal.SizeOf(typeof(RimeContext)) - sizeof(int) };
          if (RimeGetContext(session, ref ctx) == 0) {
            throw new Exception("RimeGetContext failed for " + input);
          }
          int thisPageNo = ctx.menu.page_no;
          if (seenPageNos.Contains(thisPageNo)) {
            paginationError = "page_down_did_not_advance_at_page_" + thisPageNo;
            terminationReason = "page_did_not_advance";
            RimeFreeContext(ref ctx);
            break;
          }
          if (thisPageNo != pageIndex) {
            RimeFreeContext(ref ctx);
            throw new Exception(
                "non-contiguous page_no while capturing " + input +
                ": expected " + pageIndex.ToString() +
                ", got " + thisPageNo.ToString());
          }
          seenPageNos.Add(thisPageNo);
          var pageCandidates = ReadCandidates(ctx, identity);
          foreach (var cand in pageCandidates) {
            cand["global_index"] = globalIndex++;
          }
          if (pageIndex == 0) {
            firstPageCandidates = pageCandidates;
            preedit = S(ctx.composition.preedit);
            commitPreview = S(ctx.commit_text_preview);
            rimeInput = CurrentInput(session, ctx);
            if (!string.Equals(rimeInput, input, StringComparison.Ordinal)) {
              RimeFreeContext(ref ctx);
              throw new Exception(
                  "RimeGetInput mismatch while capturing " + input +
                  ": got " + (rimeInput ?? "<null>"));
            }
            highlighted = ctx.menu.highlighted_candidate_index;
            firstPageSize = ctx.menu.page_size;
            firstPageNo = ctx.menu.page_no;
            firstPageNumCandidates = ctx.menu.num_candidates;
            firstPageIsLast = ctx.menu.is_last_page != 0;
            firstPageCandidatePointerNull = ctx.menu.candidates == IntPtr.Zero;
          }
          bool noMenu = pageIndex == 0 &&
              ctx.menu.page_no == 0 &&
              ctx.menu.page_size == 0 &&
              ctx.menu.num_candidates == 0 &&
              ctx.menu.is_last_page == 0 &&
              ctx.menu.candidates == IntPtr.Zero;
          if (noMenu) {
            menuPresent = false;
            capturedAllPages = true;
            terminationReason = "no_menu";
            RimeFreeContext(ref ctx);
            break;
          }
          var pageRec = new Dictionary<string, object>();
          pageRec["page_no"] = thisPageNo;
          pageRec["page_size"] = ctx.menu.page_size;
          pageRec["is_last_page"] = ctx.menu.is_last_page != 0;
          pageRec["candidates"] = pageCandidates;
          pages.Add(pageRec);
          allCandidates.AddRange(pageCandidates);
          bool lastPage = ctx.menu.is_last_page != 0;
          bool empty = ctx.menu.num_candidates == 0;
          RimeFreeContext(ref ctx);
          if (lastPage || empty) {
            if (lastPage) {
              capturedAllPages = true;
              terminationReason = "last_page";
            } else {
              paginationError = "empty_nonterminal_page_at_page_" + thisPageNo;
              terminationReason = "empty_nonterminal_page";
            }
            break;
          }
          int pageDownHandled = RimeProcessKey(session, PageDownKeycode, 0);
          if (pageDownHandled == 0) {
            throw new Exception(
                "Page_Down was not handled while capturing " + input +
                " at page " + thisPageNo.ToString());
          }
        }

        var result = new Dictionary<string, object>();
        result["schema_id"] = S(status.schema_id);
        result["schema_name"] = S(status.schema_name);
        result["input"] = input;
        result["rime_get_input"] = rimeInput;
        result["processed"] = processed;
        result["is_composing"] = status.is_composing != 0;
        result["is_ascii_mode"] = status.is_ascii_mode != 0;
        result["preedit"] = preedit;
        result["commit_text_preview"] = commitPreview;
        result["highlighted_candidate_index"] = highlighted;
        result["page_size"] = firstPageSize;
        result["page_no"] = firstPageNo;
        result["num_candidates"] = firstPageNumCandidates;
        result["is_last_page"] = firstPageIsLast;
        result["candidate_pointer_null"] = firstPageCandidatePointerNull;
        result["menu_present"] = menuPresent;
        result["selected_candidates"] = firstPageCandidates;
        result["pages"] = pages;
        result["all_candidates"] = allCandidates;
        result["captured_all_pages"] = capturedAllPages;
        result["termination_reason"] = terminationReason;
        if (paginationError != null) {
          result["pagination_error"] = paginationError;
        }
        results.Add(result);
        RimeFreeStatus(ref status);
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
    return CaptureScenariosWithIdentity(
        shared, user, build, schema, modulesInput, scenarios, UpstreamIdentity());
  }

  public static List<Dictionary<string, object>> CaptureScenariosWithIdentity(
      string shared,
      string user,
      string build,
      string schema,
      string[] modulesInput,
      ProbeScenario[] scenarios,
      ProbeIdentity identity) {
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
      var traits = Traits(shared, scenarioUser, build, modulesInput, identity, ptrs);
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
        foreach (var option in CaptureRuntimeOptionPolicy) {
          RimeSetOption(session, U8(option.name, ptrs), option.enabled ? 1 : 0);
        }

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
                results.Add(Snapshot(session, scenario.name, label, commit, identity));
              }
            }
          } else if (type == "key") {
            var processed = RimeProcessKey(session, action.keycode, action.mask);
            var commit = TakeCommit(session);
            if (commit != null || !string.IsNullOrEmpty(action.label)) {
              var label = string.IsNullOrEmpty(action.label)
                  ? "after_key_" + action.keycode.ToString()
                  : action.label;
              var snapshot = Snapshot(session, scenario.name, label, commit, identity);
              snapshot["processed"] = processed;
              results.Add(snapshot);
            }
          } else if (type == "set_option") {
            RimeSetOption(session, U8(action.option ?? "", ptrs), action.value);
          } else if (type == "clear") {
            RimeClearComposition(session);
          } else if (type == "snapshot") {
            results.Add(Snapshot(session, scenario.name, action.label ?? "snapshot", null, identity));
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

  public static Dictionary<string, object> CaptureSchemaListWithIdentity(
      string shared,
      string user,
      string build,
      string[] modulesInput,
      ProbeIdentity identity) {
    var ptrs = new List<IntPtr>();
    var traits = Traits(shared, user, build, modulesInput, identity, ptrs);
    var result = new Dictionary<string, object>();
    var list = new RimeSchemaList();
    try {
      RimeSetup(ref traits);
      RimeInitialize(ref traits);
      result["rime_get_schema_list"] = RimeGetSchemaList(ref list) != 0;
      result["schemas"] = ReadSchemaList(list);
      return result;
    } finally {
      if (list.list != IntPtr.Zero) {
        RimeFreeSchemaList(ref list);
      }
      RimeFinalize();
      foreach (var p in ptrs) {
        Marshal.FreeHGlobal(p);
      }
    }
  }

  public static Dictionary<string, object> CaptureStateLabelsWithIdentity(
      string shared,
      string user,
      string build,
      string schema,
      string[] modulesInput,
      ProbeIdentity identity) {
    var ptrs = new List<IntPtr>();
    var traits = Traits(shared, user, build, modulesInput, identity, ptrs);
    var result = new Dictionary<string, object>();
    UIntPtr session = UIntPtr.Zero;
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

      var status = new RimeStatus { data_size = Marshal.SizeOf(typeof(RimeStatus)) - sizeof(int) };
      if (RimeGetStatus(session, ref status) == 0) {
        throw new Exception("RimeGetStatus failed for state-label capture");
      }
      result["schema_id"] = S(status.schema_id);
      result["schema_name"] = S(status.schema_name);
      RimeFreeStatus(ref status);

      var optionName = U8("full_shape", ptrs);
      var labels = new List<Dictionary<string, object>>();
      foreach (var state in new int[] { 0, 1 }) {
        var abbreviated = RimeGetStateLabelAbbreviated(session, optionName, state, 1);
        var abbreviatedLabel = SliceS(abbreviated);
        var abbreviatedLength = checked((long)abbreviated.length.ToUInt64());
        var row = new Dictionary<string, object>();
        row["option"] = "full_shape";
        row["state"] = state;
        row["label"] = S(RimeGetStateLabel(session, optionName, state));
        row["abbreviated_label"] = abbreviatedLabel;
        row["abbreviated_length"] = abbreviatedLength;
        labels.Add(row);
      }
      result["labels"] = labels;

      RimeDestroySession(session);
      session = UIntPtr.Zero;
      return result;
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

  public static Dictionary<string, object> ProbeUserDictExportWithIdentity(
      string shared,
      string user,
      string build,
      string schema,
      string[] modulesInput,
      string input,
      string dictName,
      string exportPath,
      ProbeIdentity identity) {
    var ptrs = new List<IntPtr>();
    var traits = Traits(shared, user, build, modulesInput, identity, ptrs);
    var result = new Dictionary<string, object>();
    UIntPtr session = UIntPtr.Zero;
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
      RimeSetOption(session, U8("ascii_mode", ptrs), 0);
      foreach (var ch in input ?? "") {
        RimeProcessKey(session, (int)ch, 0);
        TakeCommit(session);
      }
      RimeProcessKey(session, 32, 0);
      result["training_input"] = input;
      result["commit_text"] = TakeCommit(session);
      RimeDestroySession(session);
      session = UIntPtr.Zero;
      result["sync_user_data"] = RimeSyncUserData() != 0;

      var modulePtr = RimeFindModule(U8("levers", ptrs));
      result["levers_module_found"] = modulePtr != IntPtr.Zero;
      if (modulePtr == IntPtr.Zero) {
        result["export_attempted"] = false;
        result["blocker"] = "RimeFindModule(\"levers\") returned null in the v1.1.2 oracle process";
        return result;
      }

      var module = (RimeModule)Marshal.PtrToStructure(modulePtr, typeof(RimeModule));
      result["levers_module_name"] = S(module.module_name);
      if (module.get_api == IntPtr.Zero) {
        result["export_attempted"] = false;
        result["blocker"] = "levers module has a null get_api function pointer";
        return result;
      }

      var getApi = Marshal.GetDelegateForFunctionPointer<GetCustomApiDelegate>(module.get_api);
      var apiPtr = getApi();
      result["levers_api_found"] = apiPtr != IntPtr.Zero;
      if (apiPtr == IntPtr.Zero) {
        result["export_attempted"] = false;
        result["blocker"] = "levers get_api returned null";
        return result;
      }

      var api = (RimeLeversApi)Marshal.PtrToStructure(apiPtr, typeof(RimeLeversApi));
      result["levers_api_data_size"] = api.data_size;
      result["export_function_found"] = api.export_user_dict != IntPtr.Zero;
      if (api.export_user_dict == IntPtr.Zero) {
        result["export_attempted"] = false;
        result["blocker"] = "RimeLeversApi.export_user_dict is null";
        return result;
      }

      var exportUserDict =
          Marshal.GetDelegateForFunctionPointer<ExportUserDictDelegate>(api.export_user_dict);
      result["export_attempted"] = true;
      result["dict_name"] = dictName;
      result["export_return"] = exportUserDict(U8(dictName, ptrs), U8(exportPath, ptrs));
      result["export_file_exists"] = File.Exists(exportPath);
      if (File.Exists(exportPath)) {
        result["export_text"] = File.ReadAllText(exportPath, Encoding.UTF8);
      }
      return result;
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

  public static Dictionary<string, object> CaptureImportedUserDictWithIdentity(
      string shared,
      string user,
      string build,
      string schema,
      string[] modulesInput,
      string dictName,
      string importPath,
      string[] inputs,
      ProbeIdentity identity) {
    var ptrs = new List<IntPtr>();
    var traits = Traits(shared, user, build, modulesInput, identity, ptrs);
    var result = new Dictionary<string, object>();
    UIntPtr session = UIntPtr.Zero;
    try {
      RimeSetup(ref traits);
      RimeInitialize(ref traits);

      var modulePtr = RimeFindModule(U8("levers", ptrs));
      result["levers_module_found"] = modulePtr != IntPtr.Zero;
      if (modulePtr == IntPtr.Zero) {
        result["import_attempted"] = false;
        result["blocker"] = "RimeFindModule(\"levers\") returned null in the v1.1.2 oracle process";
        return result;
      }

      var module = (RimeModule)Marshal.PtrToStructure(modulePtr, typeof(RimeModule));
      result["levers_module_name"] = S(module.module_name);
      if (module.get_api == IntPtr.Zero) {
        result["import_attempted"] = false;
        result["blocker"] = "levers module has a null get_api function pointer";
        return result;
      }

      var getApi = Marshal.GetDelegateForFunctionPointer<GetCustomApiDelegate>(module.get_api);
      var apiPtr = getApi();
      result["levers_api_found"] = apiPtr != IntPtr.Zero;
      if (apiPtr == IntPtr.Zero) {
        result["import_attempted"] = false;
        result["blocker"] = "levers get_api returned null";
        return result;
      }

      var api = (RimeLeversApi)Marshal.PtrToStructure(apiPtr, typeof(RimeLeversApi));
      result["levers_api_data_size"] = api.data_size;
      result["import_function_found"] = api.import_user_dict != IntPtr.Zero;
      if (api.import_user_dict == IntPtr.Zero) {
        result["import_attempted"] = false;
        result["blocker"] = "RimeLeversApi.import_user_dict is null";
        return result;
      }

      var importUserDict =
          Marshal.GetDelegateForFunctionPointer<ImportUserDictDelegate>(api.import_user_dict);
      result["import_attempted"] = true;
      result["dict_name"] = dictName;
      result["import_file_name"] = Path.GetFileName(importPath);
      result["import_file_exists"] = File.Exists(importPath);
      if (File.Exists(importPath)) {
        result["import_text"] = File.ReadAllText(importPath, Encoding.UTF8);
      }
      result["import_return"] = importUserDict(U8(dictName, ptrs), U8(importPath, ptrs));

      session = RimeCreateSession();
      if (session == UIntPtr.Zero) {
        throw new Exception("RimeCreateSession returned zero");
      }
      if (RimeSelectSchema(session, U8(schema, ptrs)) == 0) {
        throw new Exception("RimeSelectSchema failed: " + schema);
      }
      RimeSetOption(session, U8("ascii_mode", ptrs), 0);
      RimeSetOption(session, U8("full_shape", ptrs), 0);
      RimeSetOption(session, U8("ascii_punct", ptrs), 0);
      RimeSetOption(session, U8("zh_hans", ptrs), 0);

      var captures = new List<Dictionary<string, object>>();
      foreach (var input in inputs ?? Array.Empty<string>()) {
        RimeClearComposition(session);
        var processed = new List<int>();
        foreach (var ch in input) {
          processed.Add(RimeProcessKey(session, (int)ch, 0));
        }
        var snapshot = Snapshot(session, "prefer_user_phrase", input, null, identity);
        snapshot["input"] = input;
        snapshot["processed"] = processed;
        captures.Add(snapshot);
      }
      result["captures"] = captures;
      RimeDestroySession(session);
      session = UIntPtr.Zero;
      return result;
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

  public static List<Dictionary<string, object>> CaptureScenariosSingleServiceWithIdentity(
      string shared,
      string user,
      string build,
      string schema,
      string[] modulesInput,
      ProbeScenario[] scenarios,
      ProbeIdentity identity) {
    var results = new List<Dictionary<string, object>>();
    var ptrs = new List<IntPtr>();
    var traits = Traits(shared, user, build, modulesInput, identity, ptrs);
    UIntPtr session = UIntPtr.Zero;
    try {
      RimeSetup(ref traits);
      RimeInitialize(ref traits);

      foreach (var scenario in scenarios) {
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
                results.Add(Snapshot(session, scenario.name, label, commit, identity));
              }
            }
          } else if (type == "key") {
            var processed = RimeProcessKey(session, action.keycode, action.mask);
            var commit = TakeCommit(session);
            if (commit != null || !string.IsNullOrEmpty(action.label)) {
              var label = string.IsNullOrEmpty(action.label)
                  ? "after_key_" + action.keycode.ToString()
                  : action.label;
              var snapshot = Snapshot(session, scenario.name, label, commit, identity);
              snapshot["processed"] = processed;
              results.Add(snapshot);
            }
          } else if (type == "set_option") {
            RimeSetOption(session, U8(action.option ?? "", ptrs), action.value);
          } else if (type == "clear") {
            RimeClearComposition(session);
          } else if (type == "snapshot") {
            results.Add(Snapshot(session, scenario.name, action.label ?? "snapshot", null, identity));
          } else {
            throw new Exception("unsupported scenario action type: " + type);
          }
        }

        RimeDestroySession(session);
        session = UIntPtr.Zero;
      }
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
}
