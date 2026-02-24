import 'dart:async';

import 'package:flutter/material.dart';
import 'package:omniread/services/headless_renderer.dart';

class HeadlessRenderTestPage extends StatefulWidget {
  const HeadlessRenderTestPage({super.key});

  @override
  State<HeadlessRenderTestPage> createState() => _HeadlessRenderTestPageState();
}

class _HeadlessRenderTestPageState extends State<HeadlessRenderTestPage> {
  final _urlController = TextEditingController(text: 'https://example.com');
  final _jsController = TextEditingController(
    text: _defaultJs,
  );
  final _timeoutController = TextEditingController(text: '30');

  HtmlRenderer? _renderer;
  bool _isRendering = false;
  String _status = 'Idle';
  String _result = '';

  static const _defaultJs = '''
// Wait for body to have content
await new Promise((resolve) => {
  const check = () => {
    if (document.body && document.body.innerHTML.trim().length > 0) {
      resolve();
    } else {
      setTimeout(check, 100);
    }
  };
  check();
});''';

  @override
  void dispose() {
    _renderer?.cancel();
    _urlController.dispose();
    _jsController.dispose();
    _timeoutController.dispose();
    super.dispose();
  }

  Future<void> _startRender() async {
    final url = _urlController.text.trim();
    if (url.isEmpty) {
      setState(() {
        _status = 'Error: URL is empty';
      });
      return;
    }

    final timeoutSec = int.tryParse(_timeoutController.text.trim()) ?? 30;
    final jsCode = _jsController.text;

    setState(() {
      _isRendering = true;
      _status = 'Rendering...';
      _result = '';
    });

    _renderer = HtmlRenderer(url, jsCode);

    try {
      // Directly call Dart HtmlRenderer (no Rust layer).
      // Timeout is managed here (Flutter-side).
      // Both user-exit (dispose -> cancel) and timeout use the same cancel() mechanism.
      final html = await _renderer!.render().timeout(
        Duration(seconds: timeoutSec),
        onTimeout: () {
          _renderer?.cancel();
          throw TimeoutException('Render timed out after ${timeoutSec}s');
        },
      );

      if (mounted) {
        setState(() {
          _status = 'Done (${html.length} chars)';
          _result = html;
        });
      }
    } catch (e) {
      _renderer?.cancel();
      if (mounted) {
        setState(() {
          _status = 'Error: $e';
          _result = '';
        });
      }
    } finally {
      if (mounted) {
        setState(() {
          _isRendering = false;
        });
      }
    }
  }

  void _cancelRender() {
    _renderer?.cancel();
    setState(() {
      _isRendering = false;
      _status = 'Cancelled';
    });
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(title: const Text('Headless WebView Render Test')),
      body: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            TextField(
              controller: _urlController,
              decoration: const InputDecoration(
                labelText: 'URL',
                border: OutlineInputBorder(),
              ),
              enabled: !_isRendering,
            ),
            const SizedBox(height: 8),
            TextField(
              controller: _jsController,
              decoration: const InputDecoration(
                labelText: 'JS Code (async supported)',
                border: OutlineInputBorder(),
                alignLabelWithHint: true,
              ),
              maxLines: 5,
              enabled: !_isRendering,
            ),
            const SizedBox(height: 8),
            TextField(
              controller: _timeoutController,
              decoration: const InputDecoration(
                labelText: 'Timeout (seconds)',
                border: OutlineInputBorder(),
              ),
              keyboardType: TextInputType.number,
              enabled: !_isRendering,
            ),
            const SizedBox(height: 12),
            Row(
              children: [
                Expanded(
                  child: ElevatedButton(
                    onPressed: _isRendering ? null : _startRender,
                    child: const Text('Render'),
                  ),
                ),
                const SizedBox(width: 12),
                Expanded(
                  child: ElevatedButton(
                    onPressed: _isRendering ? _cancelRender : null,
                    style: ElevatedButton.styleFrom(
                      backgroundColor: Colors.red.shade100,
                    ),
                    child: const Text('Cancel'),
                  ),
                ),
              ],
            ),
            const SizedBox(height: 8),
            Text('Status: $_status',
                style: Theme.of(context).textTheme.bodyMedium),
            const Divider(),
            Expanded(
              child: SingleChildScrollView(
                child: SelectableText(
                  _result.isEmpty ? '(no result yet)' : _result,
                  style: const TextStyle(
                    fontFamily: 'monospace',
                    fontSize: 12,
                  ),
                ),
              ),
            ),
          ],
        ),
      ),
    );
  }
}
