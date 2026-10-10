//! Whether a processor is on Microsoft's lists of processors supported by Windows 11, from the name
//! Windows reports. Only names that settle it get an answer; anything else is Unknown: Fettle never
//! guesses. Written first as regular expressions in TypeScript (now retired) and matched by hand
//! here, on the name with ASCII letters lowercased; each rule says in words what it matches.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Win11Cpu {
    Supported,
    Unsupported,
    Unknown,
}

struct Name(Vec<char>);

fn word(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

impl Name {
    fn at(&self, i: usize) -> Option<char> {
        self.0.get(i).copied()
    }

    /// `text` at `i`: where it ends.
    fn lit(&self, i: usize, text: &str) -> Option<usize> {
        let mut j = i;
        for c in text.chars() {
            if self.at(j)? != c {
                return None;
            }
            j += 1;
        }
        Some(j)
    }

    /// `\s*`
    fn spaces(&self, mut i: usize) -> usize {
        while self.at(i).is_some_and(char::is_whitespace) {
            i += 1;
        }
        i
    }

    /// `\s+`
    fn spaces1(&self, i: usize) -> Option<usize> {
        let j = self.spaces(i);
        (j > i).then_some(j)
    }

    /// `\d` repeated `min` to `max` times, greedily: the digits and where they end.
    fn digits(&self, i: usize, min: usize, max: usize) -> Option<(String, usize)> {
        let mut j = i;
        while j - i < max && self.at(j).is_some_and(|c| c.is_ascii_digit()) {
            j += 1;
        }
        (j - i >= min).then(|| (self.0[i..j].iter().collect(), j))
    }

    /// `\b`
    fn boundary(&self, i: usize) -> bool {
        let before = i > 0 && word(self.0[i - 1]);
        let after = self.at(i).is_some_and(word);
        before != after
    }

    /// The leftmost start where `m` matches.
    fn find<T>(&self, m: impl Fn(usize) -> Option<T>) -> Option<T> {
        (0..=self.0.len()).find_map(m)
    }

    /// `core(?:\(tm\))?` at `i`: where it can end (with the "(tm)" first, as the regex tries it).
    fn core(&self, i: usize) -> Vec<usize> {
        let Some(j) = self.lit(i, "core") else { return vec![] };
        self.lit(j, "(tm)").into_iter().chain([j]).collect()
    }
}

pub fn win11_cpu(name: Option<&str>) -> Win11Cpu {
    use Win11Cpu::*;
    let n = Name(name.unwrap_or("").chars().map(|c| c.to_ascii_lowercase()).collect());

    // Intel: Core Ultra, and the newer Core 3/5/7 (no "i").
    let ultra = n.find(|i| {
        let a = n.lit(i, "core(tm)").map(|j| n.spaces(j)).and_then(|j| n.lit(j, "ultra")).filter(|&j| n.boundary(j));
        let b = n.lit(i, "core").and_then(|j| n.spaces1(j)).and_then(|j| n.lit(j, "ultra")).filter(|&j| n.boundary(j));
        a.or(b)
    });
    if ultra.is_some() {
        return Supported;
    }
    let core_series = n.find(|i| {
        n.core(i).into_iter().find_map(|j| {
            let j = n.spaces1(j)?;
            matches!(n.at(j)?, '3' | '5' | '7' | '9').then_some(())?;
            let j = n.spaces1(j + 1)?;
            n.digits(j, 3, 3)
        })
    });
    if core_series.is_some() {
        return Supported;
    }
    // Core 2 and the first generation (i7 CPU 920): long before Windows 11.
    let core2 = n.find(|i| n.core(i).into_iter().find_map(|j| n.lit(n.spaces(j), "2").filter(|&k| n.boundary(k))));
    let first_gen = n.find(|i| {
        if !n.boundary(i) {
            return None;
        }
        let j = n.lit(i, "i")?;
        matches!(n.at(j)?, '3' | '5' | '7' | '9').then_some(())?;
        let j = n.spaces1(j + 1)?;
        let mut j = n.spaces1(n.lit(j, "cpu")?)?;
        while n.at(j).is_some_and(|c| c.is_ascii_lowercase()) {
            j += 1;
        }
        let (_, j) = n.digits(n.spaces(j), 3, 3)?;
        n.boundary(j).then_some(())
    });
    if core2.is_some() || first_gen.is_some() {
        return Unsupported;
    }
    let intel = n.find(|i| {
        if !n.boundary(i) {
            return None;
        }
        let j = n.lit(i, "i")?;
        matches!(n.at(j)?, '3' | '5' | '7' | '9').then_some(())?;
        let (digits, mut k) = n.digits(n.lit(j + 1, "-")?, 4, 5)?;
        let start = k;
        while n.at(k).is_some_and(|c| c.is_ascii_lowercase()) {
            k += 1;
        }
        Some((digits, n.0[start..k].iter().collect::<String>()))
    });
    if let Some((digits, suffix)) = intel {
        // 10th to 14th generation mobile parts have four digits starting 10 to 14 (i7-1165G7).
        let tenth_on = digits.starts_with('1') && matches!(digits.as_bytes()[1], b'0'..=b'4');
        let generation: u32 = if digits.len() == 5 || tenth_on { digits[..2].parse().unwrap_or(0) } else { digits[..1].parse().unwrap_or(0) };
        return match generation {
            g if g >= 8 => Supported,
            7 if digits == "7820" && suffix == "hq" => Supported,
            7 if suffix.starts_with('x') => Unknown,
            _ => Unsupported,
        };
    }

    // AMD Ryzen: the 2000 series and newer, Ryzen AI and the Z handhelds.
    let ryzen_new = n.find(|i| {
        let j = n.spaces1(n.lit(i, "ryzen")?)?;
        let ai = n.lit(j, "ai").filter(|&k| n.boundary(k));
        let z = n.lit(j, "z").and_then(|k| n.digits(k, 1, 1));
        ai.map(|_| ()).or(z.map(|_| ()))
    });
    if ryzen_new.is_some() {
        return Supported;
    }
    // `(?:pro\s+)?(\d)` then three more digits: the series digit, trying "pro" first.
    let series = |j: usize| -> Option<(char, String)> {
        let with_pro = n.lit(j, "pro").and_then(|k| n.spaces1(k));
        with_pro.into_iter().chain([j]).find_map(|k| {
            let (first, k) = n.digits(k, 1, 1)?;
            let (rest, _) = n.digits(k, 3, 3)?;
            Some((first.chars().next()?, rest))
        })
    };
    let threadripper = n.find(|i| series(n.spaces1(n.lit(n.spaces1(n.lit(i, "ryzen")?)?, "threadripper")?)?));
    if let Some((first, _)) = threadripper {
        return if first == '1' { Unsupported } else { Supported };
    }
    let ryzen = n.find(|i| {
        let j = n.spaces1(n.lit(i, "ryzen")?)?;
        matches!(n.at(j)?, '3' | '5' | '7' | '9').then_some(())?;
        series(n.spaces1(j + 1)?)
    });
    if let Some((first, rest)) = ryzen {
        if first != '1' {
            return Supported;
        }
        return if rest == "600" { Unknown } else { Unsupported };
    }
    // AMD before Ryzen.
    let dash_digit = |j: usize| n.lit(j, "-").and_then(|k| n.digits(k, 1, 1));
    let tm_then_dash = |j: usize| n.lit(j, "(tm)").into_iter().chain([j]).find_map(dash_digit);
    let old_amd = n.find(|i| {
        let fx = n.boundary(i).then(|| n.lit(i, "fx")).flatten().and_then(tm_then_dash);
        let a_series = n.boundary(i).then(|| n.lit(i, "a")).flatten().and_then(|j| ["4", "6", "8", "9", "10", "12"].iter().find_map(|m| n.lit(j, m).and_then(tm_then_dash)));
        let athlon_ii = n.lit(i, "athlon").and_then(|j| n.lit(j, "(tm)").into_iter().chain([j]).find_map(|k| n.lit(n.spaces1(k)?, "ii")));
        let named = ["phenom", "sempron", "turion", "opteron"].iter().find_map(|w| n.lit(i, w));
        fx.map(|_| ()).or(a_series.map(|_| ())).or(athlon_ii.map(|_| ())).or(named.map(|_| ()))
    });
    if old_amd.is_some() {
        return Unsupported;
    }
    Unknown
}

#[cfg(test)]
mod tests {
    use super::*;
    use Win11Cpu::*;

    #[test]
    fn processors_against_microsofts_lists() {
        let cases: &[(Option<&str>, Win11Cpu)] = &[
            (Some("Intel(R) Core(TM) i5-8250U CPU @ 1.60GHz"), Supported),
            (Some("Intel(R) Core(TM) i7-10700K CPU @ 3.80GHz"), Supported),
            (Some("11th Gen Intel(R) Core(TM) i7-1165G7 @ 2.80GHz"), Supported),
            (Some("Intel(R) Core(TM) i5-1035G1 CPU @ 1.00GHz"), Supported),
            (Some("12th Gen Intel(R) Core(TM) i5-1235U"), Supported),
            (Some("13th Gen Intel(R) Core(TM) i9-13900K"), Supported),
            (Some("Intel(R) Core(TM) Ultra 7 155H"), Supported),
            (Some("Intel(R) Core(TM) 7 150U"), Supported),
            (Some("Intel(R) Core(TM) i7-7820HQ CPU @ 2.90GHz"), Supported),
            (Some("AMD Ryzen 7 9800X3D 8-Core Processor"), Supported),
            (Some("AMD Ryzen 5 2600 Six-Core Processor"), Supported),
            (Some("AMD Ryzen 5 3500U with Radeon Vega Mobile Gfx"), Supported),
            (Some("AMD Ryzen 7 PRO 4750U with Radeon Graphics"), Supported),
            (Some("AMD Ryzen AI 9 HX 370 w/ Radeon 890M"), Supported),
            (Some("AMD Ryzen Threadripper 3970X 32-Core Processor"), Supported),
            (Some("Intel(R) Core(TM) i7-7700K CPU @ 4.20GHz"), Unsupported),
            (Some("Intel(R) Core(TM) i5-6500 CPU @ 3.20GHz"), Unsupported),
            (Some("Intel(R) Core(TM) i5-2500K CPU @ 3.30GHz"), Unsupported),
            (Some("Intel(R) Core(TM) i7 CPU 920 @ 2.67GHz"), Unsupported),
            (Some("Intel(R) Core(TM)2 Duo CPU E8400 @ 3.00GHz"), Unsupported),
            (Some("AMD Ryzen 7 1700X Eight-Core Processor"), Unsupported),
            (Some("AMD Ryzen Threadripper 1950X 16-Core Processor"), Unsupported),
            (Some("AMD FX(tm)-8350 Eight-Core Processor"), Unsupported),
            (Some("AMD A10-7850K Radeon R7, 12 Compute Cores 4C+8G"), Unsupported),
            (Some("AMD Phenom(tm) II X4 965 Processor"), Unsupported),
            (Some("AMD Ryzen 5 1600 Six-Core Processor"), Unknown),
            (Some("Intel(R) Core(TM) i9-7900X CPU @ 3.30GHz"), Unknown),
            (Some("Intel(R) Pentium(R) Silver N5000 CPU @ 1.10GHz"), Unknown),
            (Some("Intel(R) Celeron(R) N4020 CPU @ 1.10GHz"), Unknown),
            (Some("Intel(R) Xeon(R) CPU E5-2680 v4 @ 2.40GHz"), Unknown),
            (Some("CPU *with* `markdown`"), Unknown),
            (Some(""), Unknown),
            (None, Unknown),
        ];
        for (name, expected) in cases {
            assert_eq!(win11_cpu(*name), *expected, "{name:?}");
        }
    }
}
