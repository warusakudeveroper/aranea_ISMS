/**
 * usePresetAnimation Hook
 *
 * Issue #107: プリセット切替時のアニメーション
 *
 * プリセットバランス値の変化をアニメーションで表現
 * - Duration: 500ms
 * - Easing: ease-out (cubic)
 */

import { useState, useCallback, useRef } from 'react';

export interface PresetBalance {
  detection_sensitivity: number;
  person_detail: number;
  object_variety: number;
  motion_sensitivity: number;
}

interface UsePresetAnimationOptions {
  duration?: number; // ms
}

// Linear interpolation
const lerp = (start: number, end: number, t: number): number => {
  return start + (end - start) * t;
};

// Ease-out cubic
const easeOutCubic = (t: number): number => {
  return 1 - Math.pow(1 - t, 3);
};

export const usePresetAnimation = (options: UsePresetAnimationOptions = {}) => {
  const { duration = 500 } = options;
  const [currentBalance, setCurrentBalance] = useState<PresetBalance>({
    detection_sensitivity: 50,
    person_detail: 50,
    object_variety: 50,
    motion_sensitivity: 80,
  });
  const [isAnimating, setIsAnimating] = useState(false);
  const animationRef = useRef<number | null>(null);

  const animateTo = useCallback((target: PresetBalance) => {
    // Cancel any existing animation
    if (animationRef.current !== null) {
      cancelAnimationFrame(animationRef.current);
    }

    setIsAnimating(true);
    const startBalance = { ...currentBalance };
    const startTime = performance.now();

    const animate = (currentTime: number) => {
      const elapsed = currentTime - startTime;
      const progress = Math.min(elapsed / duration, 1);
      const eased = easeOutCubic(progress);

      const newBalance: PresetBalance = {
        detection_sensitivity: Math.round(lerp(startBalance.detection_sensitivity, target.detection_sensitivity, eased)),
        person_detail: Math.round(lerp(startBalance.person_detail, target.person_detail, eased)),
        object_variety: Math.round(lerp(startBalance.object_variety, target.object_variety, eased)),
        motion_sensitivity: Math.round(lerp(startBalance.motion_sensitivity, target.motion_sensitivity, eased)),
      };

      setCurrentBalance(newBalance);

      if (progress < 1) {
        animationRef.current = requestAnimationFrame(animate);
      } else {
        setIsAnimating(false);
        animationRef.current = null;
      }
    };

    animationRef.current = requestAnimationFrame(animate);
  }, [currentBalance, duration]);

  const setInstant = useCallback((balance: PresetBalance) => {
    if (animationRef.current !== null) {
      cancelAnimationFrame(animationRef.current);
      animationRef.current = null;
    }
    setIsAnimating(false);
    setCurrentBalance(balance);
  }, []);

  return {
    currentBalance,
    isAnimating,
    animateTo,
    setInstant,
  };
};

export default usePresetAnimation;
