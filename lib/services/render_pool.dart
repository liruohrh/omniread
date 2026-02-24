import 'dart:async';
import 'dart:collection';

import 'headless_renderer.dart';

/// Signature for a render function: takes (url, jsCode), returns rendered HTML.
/// Used to decouple [RenderPool] from [HeadlessRenderer] for testability.
typedef RenderFn = Future<String> Function(String url, String jsCode);

/// A render task handle returned by [RenderPool.submit].
/// Use [cancel] to cancel this specific task (whether queued or running).
class RenderTask {
  final String url;
  final String jsCode;
  final Completer<String> _completer = Completer<String>();

  HeadlessRenderer? _renderer;
  bool _isCancelled = false;

  RenderTask(this.url, this.jsCode);

  /// The future that completes with the rendered HTML, or errors on
  /// failure/cancellation.
  Future<String> get future => _completer.future;

  /// Whether this task has been cancelled.
  bool get isCancelled => _isCancelled;

  /// Whether this task is still pending or running (not yet resolved).
  bool get isPending => !_completer.isCompleted;

  /// Cancel this task.
  /// - If queued (not yet started): completes with [RenderCancelledException].
  /// - If running: disposes the WebView, completes with [RenderCancelledException].
  /// - If already completed: no-op.
  void cancel() {
    if (_isCancelled || _completer.isCompleted) return;
    _isCancelled = true;
    _renderer?.cancel();
    if (!_completer.isCompleted) {
      _completer.completeError(RenderCancelledException(url));
    }
  }
}

/// Concurrent render pool that manages multiple [HeadlessRenderer] instances
/// with a configurable concurrency limit.
///
/// ## Usage Scenarios
/// - **Reading mode**: pool with maxConcurrent=2 (current chapter + prefetch)
/// - **Cache download**: pool with user-configurable maxConcurrent
///
/// Create separate pool instances for independent concurrency domains.
///
/// ## Example
/// ```dart
/// // Reading: fixed 2 concurrent
/// final readingPool = RenderPool(maxConcurrent: 2);
///
/// // Cache: user-configurable
/// final cachePool = RenderPool(maxConcurrent: userSetting);
///
/// final task = readingPool.submit('https://example.com', 'await ...');
/// final html = await task.future.timeout(Duration(seconds: 30), onTimeout: () {
///   task.cancel();
///   throw TimeoutException('timeout');
/// });
///
/// // Cancel everything on exit
/// readingPool.dispose();
/// ```
class RenderPool {
  int _maxConcurrent;
  int _running = 0;
  final _queue = Queue<RenderTask>();
  final _activeTasks = <RenderTask>{};
  bool _disposed = false;
  final RenderFn? _renderFn;

  /// Creates a RenderPool.
  ///
  /// [maxConcurrent]: max number of concurrent render tasks.
  /// [renderFn]: optional custom render function. If null, uses [HeadlessRenderer].
  ///   Inject a mock here for unit testing without a real device.
  RenderPool({required int maxConcurrent, RenderFn? renderFn})
      : _maxConcurrent = maxConcurrent,
        _renderFn = renderFn;

  /// Current max concurrency. Can be changed at runtime via [maxConcurrent=].
  int get maxConcurrent => _maxConcurrent;

  /// Update the concurrency limit at runtime.
  /// If increased, queued tasks will start immediately up to the new limit.
  /// If decreased, running tasks are NOT cancelled — they finish naturally,
  /// and no new tasks start until running count drops below the new limit.
  set maxConcurrent(int value) {
    assert(value > 0);
    _maxConcurrent = value;
    _tryNext();
  }

  /// Number of currently running render tasks.
  int get runningCount => _running;

  /// Number of tasks waiting in the queue.
  int get queuedCount => _queue.length;

  /// Submit a render task. Returns a [RenderTask] handle for cancellation.
  /// The rendered HTML is available via [RenderTask.future].
  RenderTask submit(String url, String jsCode) {
    if (_disposed) {
      throw StateError('RenderPool has been disposed');
    }
    final task = RenderTask(url, jsCode);
    _queue.add(task);
    _tryNext();
    return task;
  }

  /// Cancel all queued and running tasks.
  void cancelAll() {
    for (final task in _queue) {
      task.cancel();
    }
    _queue.clear();

    for (final task in _activeTasks.toList()) {
      task.cancel();
    }
  }

  /// Dispose the pool: cancel all tasks and reject future submissions.
  void dispose() {
    _disposed = true;
    cancelAll();
  }

  void _tryNext() {
    while (_running < _maxConcurrent && _queue.isNotEmpty) {
      final task = _queue.removeFirst();

      // Skip tasks that were cancelled while queued.
      if (task.isCancelled) continue;

      _running++;
      _activeTasks.add(task);
      _execute(task);
    }
  }

  void _execute(RenderTask task) {
    if (_renderFn != null) {
      _executeWithFn(task);
    } else {
      _executeWithRenderer(task);
    }
  }

  void _executeWithRenderer(RenderTask task) {
    final renderer = HeadlessRenderer();
    task._renderer = renderer;

    renderer.render(task.url, task.jsCode).then((html) {
      if (!task._completer.isCompleted) {
        task._completer.complete(html);
      }
    }).catchError((Object e) {
      if (!task._completer.isCompleted) {
        task._completer.completeError(e);
      }
    }).whenComplete(() {
      task._renderer = null;
      _activeTasks.remove(task);
      _running--;
      _tryNext();
    });
  }

  void _executeWithFn(RenderTask task) {
    _renderFn!(task.url, task.jsCode).then((html) {
      if (!task._completer.isCompleted) {
        task._completer.complete(html);
      }
    }).catchError((Object e) {
      if (!task._completer.isCompleted) {
        task._completer.completeError(e);
      }
    }).whenComplete(() {
      _activeTasks.remove(task);
      _running--;
      _tryNext();
    });
  }
}
