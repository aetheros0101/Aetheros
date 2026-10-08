// ============================================================
// src/task/priority.rs
//
// Faz 1 Düzeltmesi:
//
// [BUG #2] Derived PartialOrd/Ord kaldırıldı, weight() tabanlı
//          özel implementasyon eklendi.
//
// SORUN:
//   Rust enum'larda derived Ord, variant tanım sırasını kullanır:
//     Critical = 0, High = 1, Normal = 2, Low = 3
//   BinaryHeap bir max-heap'tir → en büyük değeri pop eder.
//   Sonuç: Low(3) > Critical(0) → Düşük öncelikli task'lar
//   önce çalışıyordu. Sistemin tamamı tersyüz çalışıyordu.
//
// DÜZELTME:
//   PartialOrd + Ord, weight() kullanılarak elle implement edildi.
//     Critical = 4  (en yüksek ağırlık → max-heap'ten ilk çıkar)
//     High     = 3
//     Normal   = 2
//     Low      = 1  (en düşük ağırlık → son çıkar)
//
//   queue.rs'deki QueueItem::cmp değişmez —
//   self.priority.cmp(&other.priority) artık doğru çalışıyor.
// ============================================================

use std::cmp::Ordering;

use serde::{Deserialize, Serialize};

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    // PartialOrd ve Ord artık derive edilmiyor —
    // aşağıda weight() ile elle implement ediliyor.
    Serialize,
    Deserialize,
)]
pub enum TaskPriority {
    Critical,
    High,
    Normal,
    Low,
}

impl TaskPriority {
    /// Öncelik ağırlığı. Yüksek değer → kuyrukta öne geçer.
    ///
    /// Bu değerler Ord implementasyonunun tek kaynağıdır.
    /// Sıra değiştirmek gerekirse sadece burası güncellenir.
    pub fn weight(self) -> u8 {
        match self {
            Self::Critical => 4,
            Self::High => 3,
            Self::Normal => 2,
            Self::Low => 1,
        }
    }
}

/// weight() tabanlı sıralama.
///
/// Critical(4) > High(3) > Normal(2) > Low(1)
/// BinaryHeap (max-heap) bu sırayla pop eder → Critical önce. ✓
impl Ord for TaskPriority {
    fn cmp(&self, other: &Self) -> Ordering {
        self.weight().cmp(&other.weight())
    }
}

impl PartialOrd for TaskPriority {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn critical_is_highest() {
        assert!(TaskPriority::Critical > TaskPriority::High);
        assert!(TaskPriority::Critical > TaskPriority::Normal);
        assert!(TaskPriority::Critical > TaskPriority::Low);
    }

    #[test]
    fn ordering_is_consistent() {
        let mut priorities = vec![
            TaskPriority::Low,
            TaskPriority::Critical,
            TaskPriority::Normal,
            TaskPriority::High,
        ];
        priorities.sort();
        // sort() artan sıra → Low önce, Critical son
        assert_eq!(
            priorities,
            vec![
                TaskPriority::Low,
                TaskPriority::Normal,
                TaskPriority::High,
                TaskPriority::Critical,
            ]
        );
    }
}
