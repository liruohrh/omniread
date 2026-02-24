import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter_inappwebview/flutter_inappwebview.dart';

/// Thrown when a render operation is cancelled.
class RenderCancelledException implements Exception {
  final String url;
  const RenderCancelledException(this.url);

  @override
  String toString() => 'RenderCancelledException: $url';
}

/// Thrown when a page fails to load (network error, DNS failure, etc.).
class RenderLoadException implements Exception {
  final String url;
  final String message;
  const RenderLoadException(this.url, this.message);

  @override
  String toString() => 'RenderLoadException($url): $message';
}

/// Thrown when the server responds with an HTTP error status code.
class RenderHttpException implements Exception {
  final String url;
  final int statusCode;
  final String? reasonPhrase;
  const RenderHttpException(this.url, this.statusCode, this.reasonPhrase);

  @override
  String toString() =>
      'RenderHttpException($url): $statusCode ${reasonPhrase ?? ''}';
}

/// Thrown when JavaScript execution fails.
class RenderJsException implements Exception {
  final String url;
  final Object cause;
  final String message;
  const RenderJsException(this.url, this.cause, {this.message = ""});
  @override
  String toString() =>
      'RenderJsException(${message.isNotEmpty ? "$message, " : ""}$url): $cause';
}

/// Renders a URL in a headless (invisible) system WebView, executes JavaScript
/// to wait for elements/conditions, then captures and returns the rendered HTML.
///
/// ## Cancel/Timeout Design
/// Cancel and timeout are managed by the **caller**, not by this class.
/// Both user-exit and timeout use the same unified mechanism:
///   - User exits: caller calls `cancel()` in widget's `dispose()`
///   - Timeout:    caller uses `Future.timeout()` then calls `cancel()`
/// This keeps the renderer focused on rendering, and the caller controls lifecycle.
///
/// Example usage:
/// ```dart
/// final renderer = HeadlessRenderer();
/// try {
///   final html = await renderer
///       .render('https://example.com', 'await new Promise(r => setTimeout(r, 1000))')
///       .timeout(Duration(seconds: 30), onTimeout: () {
///     renderer.cancel();
///     throw TimeoutException('Render timed out');
///   });
/// } catch (e) {
///   renderer.cancel(); // ensure cleanup
/// }
/// ```
class HeadlessRenderer {
  HeadlessInAppWebView? _webView;
  Completer<String>? _completer;
  bool _isCancelled = false;
  bool _isDisposing = false;
  String _currentUrl = '';

  /// Whether a render task is currently running.
  bool get isRendering => _completer != null && !_completer!.isCompleted;

  /// Render [url] in a headless WebView, execute [jsCode] after page loads,
  /// then return the fully rendered HTML (document.documentElement.outerHTML).
  ///
  /// [jsCode] is executed via `callAsyncJavaScript` so async/await is supported.
  /// It has no return value — it just waits for elements/conditions to appear.
  /// After [jsCode] completes, the full page HTML is captured and returned.
  Future<String> render(String url, String jsCode) async {
    if (isRendering) {
      throw StateError(
          'A render task is already running. Call cancel() first.');
    }

    _isCancelled = false;
    _isDisposing = false; // Reset for new render
    _currentUrl = url;
    _completer = Completer<String>();

    _webView = HeadlessInAppWebView(
      initialUrlRequest: URLRequest(url: WebUri(url)),
      onLoadStop: (controller, loadedUrl) async {
        if (_isCancelled) return;

        try {
          debugPrint('HeadlessRenderer: loaded $url');
          if (jsCode.isNotEmpty) {
            if (_isCancelled) return;

            // check js syntax
            //  for webview do not throw error for invalid syntax
            //. ensure platforms: android
            try {
              final jsResult = await controller.callAsyncJavaScript(
                  functionBody: "new Function(jsCode);",
                  arguments: {"jsCode": "async ()=>{ $jsCode }"});
              if (jsResult?.error != null) {
                throw jsResult!.error!;
              }
            } catch (e) {
              throw RenderJsException(url, e, message: "check js syntax");
            }
            // invoke
            final jsResult = await controller.callAsyncJavaScript(
              functionBody: jsCode,
            );
            if (jsResult?.error != null) {
              throw jsResult!.error!;
            }
          }

          if (_isCancelled) return;

          // Capture the fully rendered HTML after JS wait completes.
          final html = await controller.evaluateJavascript(
            source: 'document.documentElement.outerHTML',
          );

          if (!_completer!.isCompleted) {
            _completer!.complete(html is String ? html : html.toString());
          }
        } catch (e) {
          if (!_completer!.isCompleted) {
            _completer!.completeError(RenderJsException(url, e));
          }
        }
      },
      onReceivedError: (controller, request, error) {
        if (!_completer!.isCompleted) {
          _completer!.completeError(
            RenderLoadException(url, '${error.type}: ${error.description}'),
          );
        }
      },
      onReceivedHttpError: (controller, request, response) {
        final requestUrl = request.url.toString();
        if (requestUrl == _currentUrl && !_completer!.isCompleted) {
          _completer!.completeError(
            RenderHttpException(
                url, response.statusCode ?? 0, response.reasonPhrase),
          );
        } else {
          debugPrint(
              'HeadlessRenderer: HTTP error for $requestUrl, status=${response.statusCode}');
        }
      },
    );

    try {
      await _webView!.run();
      final result = await _completer!.future;
      return result;
    } finally {
      _dispose();
    }
  }

  /// Cancel the current render task.
  ///
  /// This is the unified cancel mechanism used by both scenarios:
  /// - User exits: widget.dispose() -> cancel()
  /// - Timeout:    Future.timeout(duration) -> cancel()
  ///
  /// Safe to call multiple times or when no task is running.
  void cancel() {
    _isCancelled = true;
    if (_completer != null && !_completer!.isCompleted) {
      _completer!.completeError(RenderCancelledException(_currentUrl));
    }
    _dispose();
  }

  void _dispose() {
    if (_isDisposing || _webView == null) return;
    _isDisposing = true;
    Future.delayed(const Duration(milliseconds: 500), () {
      try {
        _webView?.dispose();
      } finally {
        _isDisposing = false;
        _webView = null;
      }
    });
  }
}
