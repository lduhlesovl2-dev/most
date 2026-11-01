import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_hbb/models/most_reconnect_policy.dart';

void main() {
  test('recovery stops after three retries five seconds apart', () {
    final policy = MostReconnectPolicy();
    expect([for (var i = 0; i < 5; i++) policy.nextDelaySeconds()],
        [5, 5, 5, null, null]);
  });

  test('a received frame or manual retry starts a new recovery budget', () {
    final policy = MostReconnectPolicy();
    for (var i = 0; i < 3; i++) { policy.nextDelaySeconds(); }
    expect(policy.nextDelaySeconds(), isNull);
    policy.reset();
    expect(policy.nextDelaySeconds(), 5);
  });
}
