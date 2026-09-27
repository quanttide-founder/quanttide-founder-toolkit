/// memory/bloc：工作流编排（业务逻辑层）。
///
/// 事件进 → 跑 scan/judge/merge → 状态出。
/// 编排逻辑见 doc/memory.md。
library;

import '../core/engine.dart';
import '../core/rules.dart';
import 'models.dart';

/// 去向枚举。
enum Destination { insight, profile, roadmap, intention, journal }

/// 分类结果：一条日志片段的去向。
class ClassifiedEntry {
  ClassifiedEntry({required this.text, required this.destination, this.reason});

  final String text;
  final Destination destination;
  final String? reason;
}

/// 分级结果。
class GradedInsight {
  GradedInsight({required this.item, required this.grade});

  final InsightItem item;
  final InsightGrade grade;
}

/// 合并策略的四种情况。
enum MergeAction { add, rewrite, replace, crossLink }

/// 合并决策。
class MergeDecision {
  MergeDecision({required this.action, required this.target, this.source});

  final MergeAction action;
  final InsightItem target;
  final InsightItem? source;
}

/// memory 域的工作流编排。
class MemoryBloc {
  MemoryBloc({required this.engine});

  final Engine engine;

  // ── 四步流程：扫描 → 过滤 → 聚类 → 合并 ──

  /// 完整的 journal-to-* 流程：从日志条目到分层归档。
  List<ClassifiedEntry> processJournal(Workflow workflow, List<JournalEntry> entries) {
    // 1. 扫描：按关键词定位候选
    final candidates = _scanEntries(entries);

    // 2. 过滤：三问路由
    final classified = _classify(candidates);

    // 3. 聚类：同一认识跨日期合并
    final clustered = _cluster(classified);

    return clustered;
  }

  // ── 三问分层路由 ──

  /// 三问过滤：判断一条文本的去向。
  ///
  /// 「想通了什么」→ insight、「我是什么样的人」→ profile、
  /// 「我要做什么」→ roadmap、「我到底想要什么」→ intention、
  /// 「发生了什么」→ 留 journal。
  Destination route(String text) {
    // 意图类关键词优先（要/不要/别/澄清），因为意图是最明确的指令
    if (_matchesAny(text, _intentKeywords)) return Destination.intention;
    // 方向类关键词（决定/优先/待定）
    if (_matchesAny(text, _directionKeywords)) return Destination.roadmap;
    // 认知类关键词（发现/原来/其实/关键是）
    if (_matchesAny(text, _cognitionKeywords)) return Destination.insight;
    // 特征类关键词（我总是/我习惯/我害怕）
    if (_matchesAny(text, _profileKeywords)) return Destination.profile;
    // 默认留 journal
    return Destination.journal;
  }

  /// 批量分类。
  List<ClassifiedEntry> classify(List<String> texts) {
    return texts.map((t) => ClassifiedEntry(text: t, destination: route(t))).toList();
  }

  // ── 证据分级 ──

  /// 分级：多日重复或有验证 → 已确认；单次 → 假说；被推翻 → 移除。
  InsightGrade grade(String text, {int occurrenceCount = 1, bool hasVerification = false, bool isRefuted = false}) {
    if (isRefuted) return InsightGrade.hypothesis; // 被推翻的降级，由合并阶段移除
    if (occurrenceCount >= 2 || hasVerification) return InsightGrade.confirmed;
    return InsightGrade.hypothesis;
  }

  // ── 聚类 ──

  /// 同一认识跨日期合并：判断依据是本质相同而非措辞相似。
  ///
  /// 简化实现：按语义相似度判断，实际由 LLM 执行（engine.judge）。
  List<InsightItem> cluster(List<InsightItem> items) {
    final result = <InsightItem>[];
    final used = <int>{};

    for (var i = 0; i < items.length; i++) {
      if (used.contains(i)) continue;
      var current = items[i];

      for (var j = i + 1; j < items.length; j++) {
        if (used.contains(j)) continue;
        // 本质相同 → 合并；不同 → 各自保留
        if (_isSameEssence(current, items[j])) {
          current = _mergeInsightItems(current, items[j]);
          used.add(j);
        }
      }

      result.add(current);
      used.add(i);
    }

    return result;
  }

  // ── 合并策略 ──

  /// 四种情况四种处理：新增 / 就地改写 / 被推翻替换 / 重复留互链。
  MergeDecision decideMerge(InsightItem? existing, InsightItem incoming) {
    if (existing == null) {
      return MergeDecision(action: MergeAction.add, target: incoming);
    }
    if (_isRefuted(existing, incoming)) {
      return MergeDecision(action: MergeAction.replace, target: incoming, source: existing);
    }
    if (_isSameEssence(existing, incoming)) {
      return MergeDecision(action: MergeAction.rewrite, target: _mergeInsightItems(existing, incoming), source: existing);
    }
    return MergeDecision(action: MergeAction.crossLink, target: existing, source: incoming);
  }

  /// 批量合并到现有列表。
  List<InsightItem> merge(List<InsightItem> existing, List<InsightItem> incoming) {
    final result = List<InsightItem>.from(existing);
    for (final item in incoming) {
      // 找同本质的已有条目
      InsightItem? matched;
      for (final e in result) {
        if (_isSameEssence(e, item)) {
          matched = e;
          break;
        }
      }
      final decision = decideMerge(matched, item);
      switch (decision.action) {
        case MergeAction.add:
          result.add(item);
        case MergeAction.rewrite:
          result[result.indexOf(matched!)] = decision.target;
        case MergeAction.replace:
          result[result.indexOf(matched!)] = decision.target;
        case MergeAction.crossLink:
          break; // 重复的留互链，不重复收录
      }
    }
    return result;
  }

  // ── 升降级 ──

  /// 判断是否应晋升到 profile（命题反复套用、稳定为思维框架）。
  bool shouldPromoteToProfile(InsightItem item, {int applicationCount = 0}) {
    return applicationCount >= 3; // 反复套用 = 晋升
  }

  /// 判断是否应分流到 roadmap（命题引出选择）。
  bool shouldForkToRoadmap(InsightItem item) {
    return item.detail.contains('应该') || item.detail.contains('需要') || item.detail.contains('决定');
  }

  // ── 内部：四步流程的各步 ──

  /// 第一步：扫描——按关键词定位候选。
  List<String> _scanEntries(List<JournalEntry> entries) {
    final candidates = <String>[];
    for (final entry in entries) {
      for (final segment in entry.segments) {
        for (final line in segment.split('\n')) {
          final trimmed = line.trim();
          if (trimmed.isEmpty) continue;
          if (_matchesAny(trimmed, _allKeywords)) {
            candidates.add(trimmed);
          }
        }
      }
    }
    return candidates;
  }

  /// 第二步：过滤——三问路由。
  List<ClassifiedEntry> _classify(List<String> texts) {
    return classify(texts);
  }

  /// 第三步：聚类——同一认识合并。
  List<ClassifiedEntry> _cluster(List<ClassifiedEntry> entries) {
    // 按去向分组，同组内聚类
    final byDest = <Destination, List<ClassifiedEntry>>{};
    for (final e in entries) {
      byDest.putIfAbsent(e.destination, () => []).add(e);
    }
    final result = <ClassifiedEntry>[];
    for (final group in byDest.values) {
      result.addAll(group); // 简化：不跨条目聚类，由上层用 cluster() 处理
    }
    return result;
  }

  // ── 内部：判断工具 ──

  bool _matchesAny(String text, List<String> keywords) {
    return keywords.any(text.contains);
  }

  bool _isSameEssence(InsightItem a, InsightItem b) {
    // 简化：判断命题是否本质相同——实际由 LLM 判断
    // 这里用关键词重叠度做粗筛
    final aWords = a.statement.split(RegExp(r'[，。、\s]+')).where((w) => w.length >= 2).toSet();
    final bWords = b.statement.split(RegExp(r'[，。、\s]+')).where((w) => w.length >= 2).toSet();
    if (aWords.isEmpty || bWords.isEmpty) return false;
    final overlap = aWords.intersection(bWords).length;
    return overlap / aWords.length >= 0.5;
  }

  bool _isRefuted(InsightItem existing, InsightItem incoming) {
    // 被推翻 = 新条目明确否定旧条目
    return incoming.statement.contains('不是') && existing.statement.contains(incoming.statement.replaceAll('不是', '').trim());
  }

  InsightItem _mergeInsightItems(InsightItem a, InsightItem b) {
    return InsightItem(
      statement: a.statement,
      detail: '${a.detail}；${b.detail}',
      evidence: a.evidence ?? b.evidence,
    );
  }

  // ── 关键词表（来自 doc/memory.md）──

  static const _cognitionKeywords = ['发现', '意识', '原来', '其实', '关键是', '临界点', '原因在于', '实际上', '规律'];
  static const _intentKeywords = ['要', '不要', '别', '希望', '应该', '必须', '交给', '按', '先', '只', '统一', '澄清'];
  static const _directionKeywords = ['决定', '决定不', '先做', '优先', '不动', '合并', '拆开', '待定', '问题是'];
  static const _profileKeywords = ['我总是', '我习惯', '我害怕', '我反复', '我如何判断'];
  static const _allKeywords = [..._cognitionKeywords, ..._intentKeywords, ..._directionKeywords, ..._profileKeywords];
}
