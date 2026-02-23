import 'dart:async';

import 'package:flutter/material.dart';
import 'package:omniread/services/render_pool.dart';

class PoolRenderTestPage extends StatefulWidget {
  const PoolRenderTestPage({super.key});

  @override
  State<PoolRenderTestPage> createState() => _PoolRenderTestPageState();
}

class _PoolRenderTestPageState extends State<PoolRenderTestPage> {
  final _concurrencyController = TextEditingController(text: '2');
  final _timeoutController = TextEditingController(text: '30');
  final _jsController = TextEditingController(text: _defaultJs);

  // Each line is one URL to render concurrently.
  final _urlsController = TextEditingController(
    text: 'https://example.com\n'
        'https://www.iana.org/domains/reserved\n'
        'https://httpbin.org/html',
  );

  RenderPool? _pool;
  final _tasks = <_TaskEntry>[];

  static const _defaultJs = '''
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
    _pool?.dispose();
    _concurrencyController.dispose();
    _timeoutController.dispose();
    _jsController.dispose();
    _urlsController.dispose();
    super.dispose();
  }

  void _startBatch() {
    final urls = _urlsController.text
        .split('\n')
        .map((l) => l.trim())
        .where((l) => l.isNotEmpty)
        .toList();
    if (urls.isEmpty) return;

    final concurrency = int.tryParse(_concurrencyController.text.trim()) ?? 2;
    final timeoutSec = int.tryParse(_timeoutController.text.trim()) ?? 30;
    final jsCode = _jsController.text;

    _pool?.dispose();
    _pool = RenderPool(maxConcurrent: concurrency);

    setState(() {
      _tasks.clear();
    });

    for (final url in urls) {
      final task = _pool!.submit(url, jsCode);
      final entry = _TaskEntry(url: url, task: task);
      _tasks.add(entry);

      task.future.timeout(
        Duration(seconds: timeoutSec),
        onTimeout: () {
          task.cancel();
          throw TimeoutException('Timeout after ${timeoutSec}s');
        },
      ).then((html) {
        if (mounted) {
          setState(() {
            entry.status = 'Done (${html.length} chars)';
            entry.result = html;
          });
        }
      }).catchError((Object e) {
        if (mounted) {
          setState(() {
            entry.status = 'Error: $e';
          });
        }
      });

      setState(() {
        entry.status = 'Queued / Running...';
      });
    }
  }

  void _cancelAll() {
    _pool?.cancelAll();
    setState(() {
      for (final entry in _tasks) {
        if (!entry.status.startsWith('Done') &&
            !entry.status.startsWith('Error')) {
          entry.status = 'Cancelled';
        }
      }
    });
  }

  void _cancelTask(int index) {
    _tasks[index].task.cancel();
    setState(() {
      if (!_tasks[index].status.startsWith('Done')) {
        _tasks[index].status = 'Cancelled';
      }
    });
  }

  @override
  Widget build(BuildContext context) {
    final poolInfo = _pool != null
        ? 'Running: ${_pool!.runningCount}, Queued: ${_pool!.queuedCount}'
        : 'No pool';

    return Scaffold(
      appBar: AppBar(title: const Text('RenderPool Concurrent Test')),
      body: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Row(
              children: [
                Expanded(
                  child: TextField(
                    controller: _concurrencyController,
                    decoration: const InputDecoration(
                      labelText: 'Max Concurrent',
                      border: OutlineInputBorder(),
                    ),
                    keyboardType: TextInputType.number,
                  ),
                ),
                const SizedBox(width: 8),
                Expanded(
                  child: TextField(
                    controller: _timeoutController,
                    decoration: const InputDecoration(
                      labelText: 'Timeout (s)',
                      border: OutlineInputBorder(),
                    ),
                    keyboardType: TextInputType.number,
                  ),
                ),
              ],
            ),
            const SizedBox(height: 8),
            TextField(
              controller: _urlsController,
              decoration: const InputDecoration(
                labelText: 'URLs (one per line)',
                border: OutlineInputBorder(),
                alignLabelWithHint: true,
              ),
              maxLines: 3,
            ),
            const SizedBox(height: 8),
            TextField(
              controller: _jsController,
              decoration: const InputDecoration(
                labelText: 'JS Code',
                border: OutlineInputBorder(),
                alignLabelWithHint: true,
              ),
              maxLines: 3,
            ),
            const SizedBox(height: 8),
            Row(
              children: [
                Expanded(
                  child: ElevatedButton(
                    onPressed: _startBatch,
                    child: const Text('Start Batch'),
                  ),
                ),
                const SizedBox(width: 8),
                Expanded(
                  child: ElevatedButton(
                    onPressed: _tasks.isEmpty ? null : _cancelAll,
                    style: ElevatedButton.styleFrom(
                      backgroundColor: Colors.red.shade100,
                    ),
                    child: const Text('Cancel All'),
                  ),
                ),
              ],
            ),
            const SizedBox(height: 8),
            Text(poolInfo, style: Theme.of(context).textTheme.bodySmall),
            const Divider(),
            Expanded(
              child: ListView.builder(
                itemCount: _tasks.length,
                itemBuilder: (context, index) {
                  final entry = _tasks[index];
                  return Card(
                    child: ExpansionTile(
                      title: Text(
                        entry.url,
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                      ),
                      subtitle: Text(entry.status),
                      trailing: entry.task.isPending
                          ? IconButton(
                              icon: const Icon(Icons.cancel, size: 20),
                              onPressed: () => _cancelTask(index),
                            )
                          : null,
                      children: [
                        if (entry.result != null)
                          Padding(
                            padding: const EdgeInsets.all(8),
                            child: SelectableText(
                              entry.result!.length > 2000
                                  ? '${entry.result!.substring(0, 2000)}...(truncated)'
                                  : entry.result!,
                              style: const TextStyle(
                                fontFamily: 'monospace',
                                fontSize: 11,
                              ),
                            ),
                          ),
                      ],
                    ),
                  );
                },
              ),
            ),
          ],
        ),
      ),
    );
  }
}

class _TaskEntry {
  final String url;
  final RenderTask task;
  String status;
  String? result;

  _TaskEntry({required this.url, required this.task}) : status = 'Pending';
}
