import 'package:flutter/animation.dart';

class AppMotion {
  AppMotion._();

  static const Duration fast = Duration(milliseconds: 140);
  static const Duration normal = Duration(milliseconds: 220);
  static const Duration slow = Duration(milliseconds: 420);
  static const Duration pulse = Duration(milliseconds: 1200);

  static const Curve emphasized = Curves.easeOutCubic;
  static const Curve enter = Curves.easeOutQuart;
  static const Curve exit = Curves.easeInCubic;
}
