import 'package:flutter/material.dart';

import '../../../../../core/theme/app_colors.dart';
import '../../../../../core/theme/app_spacing.dart';
import '../../../../../core/theme/app_typography.dart';

class FilterOption<T> {
  const FilterOption({required this.value, required this.label, this.subtitle});

  final T value;
  final String label;
  final String? subtitle;
}

Future<void> showFilterBottomSheet<T>({
  required BuildContext context,
  required String title,
  required List<FilterOption<T>> options,
  required bool Function(T value) isSelected,
  ValueChanged<bool>? onSelectAll,
  ValueChanged<bool>? onSelectNone,
  required Future<void> Function(T value, bool selected) onToggle,
}) {
  return showModalBottomSheet<void>(
    context: context,
    isScrollControlled: true,
    backgroundColor: Colors.transparent,
    builder: (sheetContext) {
      return SafeArea(
        top: false,
        child: Padding(
          padding: const EdgeInsets.all(AppSpacing.sm),
          child: Container(
            constraints: BoxConstraints(
              maxHeight: MediaQuery.sizeOf(context).height * 0.78,
            ),
            padding: const EdgeInsets.all(AppSpacing.lg),
            decoration: BoxDecoration(
              color: AppColors.surfaceElevated.withValues(alpha: 0.88),
              borderRadius: BorderRadius.circular(24),
              border: Border.all(
                color: AppColors.accentPrimary.withValues(alpha: 0.45),
              ),
              boxShadow: [
                BoxShadow(
                  color: AppColors.accentSecondary.withValues(alpha: 0.25),
                  blurRadius: 30,
                  spreadRadius: -10,
                ),
              ],
            ),
            child: Column(
              mainAxisSize: MainAxisSize.min,
              children: [
                Row(
                  children: [
                    Expanded(
                      child: Text(
                        title,
                        style: AppTypography.textTheme.titleLarge?.copyWith(
                          color: AppColors.textPrimary,
                        ),
                      ),
                    ),
                    if (onSelectAll != null)
                      _ActionMiniButton(
                        label: 'ALL',
                        onTap: () => onSelectAll(true),
                      ),
                    if (onSelectAll != null && onSelectNone != null)
                      const SizedBox(width: AppSpacing.sm),
                    if (onSelectNone != null)
                      _ActionMiniButton(
                        label: 'NONE',
                        onTap: () => onSelectNone(true),
                      ),
                  ],
                ),
                const SizedBox(height: AppSpacing.md),
                Flexible(
                  child: StatefulBuilder(
                    builder: (context, setState) {
                      return ListView.separated(
                        itemCount: options.length,
                        separatorBuilder: (_, _) =>
                            const SizedBox(height: AppSpacing.xs),
                        itemBuilder: (_, index) {
                          final option = options[index];
                          final selected = isSelected(option.value);
                          return CheckboxListTile(
                            dense: true,
                            value: selected,
                            checkboxShape: RoundedRectangleBorder(
                              borderRadius: BorderRadius.circular(4),
                            ),
                            controlAffinity: ListTileControlAffinity.leading,
                            activeColor: AppColors.accentPrimary,
                            checkColor: AppColors.background,
                            contentPadding: const EdgeInsets.symmetric(
                              horizontal: AppSpacing.sm,
                            ),
                            shape: RoundedRectangleBorder(
                              borderRadius: BorderRadius.circular(12),
                              side: BorderSide(
                                color: selected
                                    ? AppColors.accentPrimary.withValues(
                                        alpha: 0.75,
                                      )
                                    : AppColors.textMuted.withValues(
                                        alpha: 0.28,
                                      ),
                              ),
                            ),
                            tileColor: AppColors.surface.withValues(
                              alpha: 0.68,
                            ),
                            title: Text(
                              option.label,
                              style: AppTypography.textTheme.bodyMedium
                                  ?.copyWith(color: AppColors.textPrimary),
                            ),
                            subtitle: option.subtitle == null
                                ? null
                                : Text(
                                    option.subtitle!,
                                    style: AppTypography.textTheme.bodySmall,
                                  ),
                            onChanged: (checked) async {
                              await onToggle(option.value, checked ?? false);
                              if (context.mounted) {
                                setState(() {});
                              }
                            },
                          );
                        },
                      );
                    },
                  ),
                ),
                const SizedBox(height: AppSpacing.sm),
                SizedBox(
                  width: double.infinity,
                  height: 48,
                  child: OutlinedButton(
                    onPressed: () => Navigator.of(sheetContext).pop(),
                    child: const Text('CLOSE'),
                  ),
                ),
              ],
            ),
          ),
        ),
      );
    },
  );
}

class _ActionMiniButton extends StatelessWidget {
  const _ActionMiniButton({required this.label, required this.onTap});

  final String label;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    return InkWell(
      borderRadius: BorderRadius.circular(11),
      onTap: onTap,
      child: Ink(
        padding: const EdgeInsets.symmetric(
          horizontal: AppSpacing.md,
          vertical: AppSpacing.sm,
        ),
        decoration: BoxDecoration(
          color: AppColors.surface.withValues(alpha: 0.8),
          borderRadius: BorderRadius.circular(11),
          border: Border.all(
            color: AppColors.accentSecondary.withValues(alpha: 0.8),
          ),
        ),
        child: Text(
          label,
          style: AppTypography.textTheme.labelMedium?.copyWith(
            color: AppColors.accentSecondary,
          ),
        ),
      ),
    );
  }
}
