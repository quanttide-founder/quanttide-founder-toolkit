/// fiction/bloc：工作流编排（业务逻辑层）。
///
/// 素材流向、三步提炼、章节编号轴、阶段流转、包装文案。
/// 编排逻辑见 doc/fiction.md。
library;

import '../core/engine.dart';
import 'models.dart';

/// 提炼结果：母题卡片 + 场景素材。
class ExtractedFragment {
  ExtractedFragment({required this.motif, required this.scene, required this.source});

  final String motif;  // 母题卡片：一句话说清写什么
  final String scene;  // 场景素材：50 字钩子
  final String source; // 来源素材标题
}

/// 编号分配结果。
class NumberAssignment {
  NumberAssignment({required this.assigned, this.gap});

  final int assigned; // 分配到的编号
  final int? gap;     // 填补的空号（null = 追加到最大号 +1）
}

/// 阶段完成状态。
class StageStatus {
  StageStatus({required this.isComplete, required this.missingNumbers, required this.totalChapters});

  final bool isComplete;
  final List<int> missingNumbers;
  final int totalChapters;
}

/// fiction 域的工作流编排。
class FictionBloc {
  FictionBloc({required this.engine});

  final Engine engine;

  // ── 三步提炼：取样 → 观察展开 → 落实片段 ──

  /// 从情绪素材提炼创作片段。
  ///
  /// 实验教训：直接从情绪跳到场景素材会产出「金句」而不是「画面」——
  /// 中间必须有「观察展开」：先提取具体场景、动作、细节，再长出片段。
  ExtractedFragment extract(EmotionalDiary diary) {
    // 1. 取样：从情绪记录定位可写的素材
    final sample = _sample(diary.content);

    // 2. 观察展开：从情绪提取外部观察（场景/动作/细节）
    final observation = _expandObservation(sample);

    // 3. 落实片段：观察 → 场景素材 + 母题卡片
    return _toFragment(observation, diary.title);
  }

  /// 批量提炼。
  List<ExtractedFragment> extractAll(List<EmotionalDiary> diaries) {
    return diaries.map(extract).toList();
  }

  // ── 章节编号轴 ──

  /// 分配新章节编号：优先填补空号（宁空勿移），否则当前最大号 +1。
  NumberAssignment assignNumber(Novel novel) {
    final used = novel.allChapters
        .where((c) => c.number != null && !c.isPreface)
        .map((c) => c.number!)
        .toSet();

    // 找最小空号
    if (used.isNotEmpty) {
      final max = used.reduce((a, b) => a > b ? a : b);
      for (var i = 1; i <= max; i++) {
        if (!used.contains(i)) {
          return NumberAssignment(assigned: i, gap: i);
        }
      }
      return NumberAssignment(assigned: max + 1);
    }
    return NumberAssignment(assigned: 1);
  }

  /// 检查编号轴：返回预留空号列表。
  List<int> findGaps(Novel novel) {
    final used = novel.allChapters
        .where((c) => c.number != null && !c.isPreface)
        .map((c) => c.number!)
        .toSet();
    if (used.isEmpty) return [];

    final max = used.reduce((a, b) => a > b ? a : b);
    final gaps = <int>[];
    for (var i = 1; i <= max; i++) {
      if (!used.contains(i)) gaps.add(i);
    }
    return gaps;
  }

  // ── 阶段流转 ──

  /// 检查阶段完成度：该阶段的章节都已编号且无空位。
  StageStatus checkStage(Stage stage) {
    final numbered = stage.chapters.where((c) => c.number != null && !c.isPreface).toList();
    final used = numbered.map((c) => c.number!).toSet();

    final gaps = <int>[];
    if (used.isNotEmpty) {
      final max = used.reduce((a, b) => a > b ? a : b);
      for (var i = 1; i <= max; i++) {
        if (!used.contains(i)) gaps.add(i);
      }
    }

    return StageStatus(
      isComplete: gaps.isEmpty && numbered.isNotEmpty,
      missingNumbers: gaps,
      totalChapters: stage.chapters.length,
    );
  }

  /// 发现小说的阶段流转方向：按编号排序，返回阶段名列表。
  List<String> stageFlow(Novel novel) {
    return novel.stages.map((s) => '${s.number}_${s.name}').toList();
  }

  // ── 包装文案（晋江规范）──

  /// 从正文提取包装文案：标题、一句话简介、立意。
  ///
  /// 三类文案各自承担不同信息，不互相重复：
  /// 标题 = 核心矛盾与钩子；一句话简介 = 增量信息（≤15 字）；立意 = 价值维度。
  Map<String, String> extractPackaging(String content) {
    // 从正文提取具体细节（台词、场景、意象）做钩子
    final sentences = content.split(RegExp(r'[。！？\n]')).where((s) => s.trim().isNotEmpty).toList();

    // 标题：找有画面感或反转的长句
    String title = sentences.isNotEmpty ? sentences.first.trim() : '';

    // 一句话简介：回答「凭什么看」，不超过 15 字
    String tagline = '';
    for (final s in sentences) {
      final len = s.trim().length;
      if (len >= 5 && len <= 15) {
        tagline = s.trim();
        break;
      }
    }

    // 立意：正向，与文案结尾呼应
    String theme = sentences.isNotEmpty ? sentences.last.trim() : '';

    return {'title': title, 'tagline': tagline, 'theme': theme};
  }

  // ── 内部：三步提炼的各步 ──

  /// 第一步：取样——从情绪记录定位可写素材。
  String _sample(String content) {
    // 找有具体场景/动作的句子
    final lines = content.split('\n').where((l) => l.trim().isNotEmpty);
    for (final line in lines) {
      if (_hasConcreteDetail(line)) return line.trim();
    }
    return content.trim();
  }

  /// 第二步：观察展开——从情绪提取外部观察。
  String _expandObservation(String sample) {
    // 提取具体场景、动作、细节，不是抽象感受
    // 简化实现：保留含具体细节的片段
    return sample;
  }

  /// 第三步：落实片段——观察 → 场景素材 + 母题卡片。
  ExtractedFragment _toFragment(String observation, String sourceTitle) {
    // 母题卡片：一句话说清写什么
    final motif = sourceTitle;

    // 场景素材：50 字钩子
    final scene = observation.length > 50 ? '${observation.substring(0, 50)}…' : observation;

    return ExtractedFragment(motif: motif, scene: scene, source: sourceTitle);
  }

  /// 判断文本是否含具体细节（场景/动作/物件）。
  bool _hasConcreteDetail(String text) {
    const detailMarkers = ['拿', '递', '坐', '站', '走', '看', '说', '笑', '哭', '吃', '喝', '买', '修'];
    return detailMarkers.any(text.contains);
  }
}
