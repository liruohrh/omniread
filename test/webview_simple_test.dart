import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_inappwebview/flutter_inappwebview.dart';

/// Simple test page for flutter_inappwebview to isolate crash source.
/// Tests both headed (InAppWebView widget) and headless rendering.
class WebViewSimpleTestPage extends StatefulWidget {
  const WebViewSimpleTestPage({super.key});

  @override
  State<WebViewSimpleTestPage> createState() => _WebViewSimpleTestPageState();
}

class _WebViewSimpleTestPageState extends State<WebViewSimpleTestPage> {
  String _headedStatus = 'Idle';
  String _headlessStatus = 'Idle';
  String _headedResult = '';
  String _headlessResult = '';

  InAppWebViewController? _headedController;
  HeadlessInAppWebView? _headlessWebView;

  static const _testUrl = 'https://example.com';
  static const _testJs = 'return document.title;';

  @override
  void dispose() {
    _headlessWebView?.dispose();
    super.dispose();
  }

  // ============ Headed WebView Test ============
  Future<void> _testHeadedJs() async {
    if (_headedController == null) {
      setState(() => _headedStatus = 'Error: WebView not ready');
      return;
    }

    setState(() {
      _headedStatus = 'Running JS...';
      _headedResult = '';
    });

    try {
      final result = await _headedController!.callAsyncJavaScript(
        functionBody: _testJs,
      );
      setState(() {
        _headedStatus = 'Done';
        _headedResult = 'Result: ${result?.value}';
      });
    } catch (e) {
      setState(() {
        _headedStatus = 'Error: $e';
        _headedResult = '';
      });
    }
  }

  // ============ Headless WebView Test ============
  Future<void> _testHeadless() async {
    setState(() {
      _headlessStatus = 'Creating headless webview...';
      _headlessResult = '';
    });

    final completer = Completer<String>();

    _headlessWebView?.dispose();
    _headlessWebView = HeadlessInAppWebView(
      initialUrlRequest: URLRequest(url: WebUri(_testUrl)),
      onLoadStop: (controller, url) async {
        if (completer.isCompleted) return;

        setState(() => _headlessStatus = 'Page loaded, running JS...');

        try {
          final result = await controller.callAsyncJavaScript(
            functionBody: _testJs,
          );
          if (!completer.isCompleted) {
            completer.complete('Result: ${result?.value}');
          }
        } catch (e) {
          if (!completer.isCompleted) {
            completer.completeError(e);
          }
        }
      },
      onReceivedError: (controller, request, error) {
        if (!completer.isCompleted) {
          completer.completeError('${error.type}: ${error.description}');
        }
      },
    );

    try {
      await _headlessWebView!.run();
      final result = await completer.future.timeout(
        const Duration(seconds: 30),
        onTimeout: () => throw TimeoutException('Timeout'),
      );
      setState(() {
        _headlessStatus = 'Done';
        _headlessResult = result;
      });
    } catch (e) {
      setState(() {
        _headlessStatus = 'Error: $e';
        _headlessResult = '';
      });
    } finally {
      _headlessWebView?.dispose();
      _headlessWebView = null;
    }
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(title: const Text('WebView Simple Test')),
      body: Column(
        children: [
          // Headed WebView Section
          Container(
            height: 200,
            margin: const EdgeInsets.all(8),
            decoration: BoxDecoration(
              border: Border.all(color: Colors.blue),
            ),
            child: InAppWebView(
              initialUrlRequest: URLRequest(url: WebUri(_testUrl)),
              onWebViewCreated: (controller) {
                _headedController = controller;
              },
              onLoadStop: (controller, url) {
                setState(() => _headedStatus = 'Ready');
              },
            ),
          ),

          // Headed Controls
          Padding(
            padding: const EdgeInsets.symmetric(horizontal: 8),
            child: Row(
              children: [
                const Text('Headed: ',
                    style: TextStyle(fontWeight: FontWeight.bold)),
                Expanded(child: Text(_headedStatus)),
                ElevatedButton(
                  onPressed: _testHeadedJs,
                  child: const Text('Run JS'),
                ),
              ],
            ),
          ),
          if (_headedResult.isNotEmpty)
            Padding(
              padding: const EdgeInsets.all(8),
              child: Text(_headedResult,
                  style: const TextStyle(color: Colors.green)),
            ),

          const Divider(height: 32),

          // Headless Controls
          Padding(
            padding: const EdgeInsets.symmetric(horizontal: 8),
            child: Row(
              children: [
                const Text('Headless: ',
                    style: TextStyle(fontWeight: FontWeight.bold)),
                Expanded(child: Text(_headlessStatus)),
                ElevatedButton(
                  onPressed: _testHeadless,
                  child: const Text('Test'),
                ),
              ],
            ),
          ),
          if (_headlessResult.isNotEmpty)
            Padding(
              padding: const EdgeInsets.all(8),
              child: Text(_headlessResult,
                  style: const TextStyle(color: Colors.green)),
            ),

          const Spacer(),

          // Info
          const Padding(
            padding: EdgeInsets.all(16),
            child: Text(
              'This page tests raw flutter_inappwebview without any wrapper.\n'
              'Compare headed vs headless to isolate crash source.',
              style: TextStyle(color: Colors.grey, fontSize: 12),
              textAlign: TextAlign.center,
            ),
          ),
        ],
      ),
    );
  }
}
