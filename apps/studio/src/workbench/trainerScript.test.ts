import { describe, expect, it } from "vitest";
import type { DataRowDto } from "../api";
import { placeableTrainers, placedLabel, scriptTrainerLabel, trainerLabel, trainerOfScript, trainerPatch, trainerScript } from "./trainerScript";

const row = (index: number, name: string, cls: string | null): DataRowDto => ({
  index,
  name,
  tech: String(index).padStart(4, "0"),
  locked: cls === null ? "placeholder for the sight script slot" : null,
  extra: cls === null ? null : { class: 1, class_name: cls, level_max: 5, party_count: 1, places: [] },
});
const rows = [row(0, "", "Klasse 0"), row(4, "Michael", "Teenager"), row(356, "Sebastian", "Teenager"), row(928, "Jens", "Wanderer"), row(929, "", null), row(930, "Mia", "Göre")];

describe("trainer scripts", () => {
  it("reads_the_trainer_from_3000_to_4999_and_a_partner_from_5000", () => {
    expect(trainerOfScript(3355)).toEqual({ id: 356, partner: false });
    expect(trainerOfScript(3000)).toEqual({ id: 1, partner: false });
    expect(trainerOfScript(4999)).toEqual({ id: 2000, partner: false });
    expect(trainerOfScript(5003)).toEqual({ id: 4, partner: true });
    expect(trainerOfScript(2999)).toBeNull();
    expect(trainerOfScript(0)).toBeNull();
    expect(trainerOfScript(7000)).toBeNull();
    expect(trainerScript(356)).toBe(3355);
    expect(trainerScript(930)).toBe(3929);
  });

  it("sets_the_script_and_turns_the_sightline_on_only_when_type_and_sight_are_both_0", () => {
    expect(trainerPatch(930, { trainer_type: 0, sight: 0 })).toEqual({ script: 3929, trainer_type: 1, sight: 3 });
    expect(trainerPatch(930, { trainer_type: 1, sight: 5 })).toEqual({ script: 3929 });
    expect(trainerPatch(930, { trainer_type: 0, sight: 2 })).toEqual({ script: 3929 });
    // A double battle partner stays a partner.
    expect(trainerPatch(930, { trainer_type: 1, sight: 5 }, true)).toEqual({ script: 5929 });
    expect(trainerScript(4, true)).toBe(5003);
  });

  it("offers_every_trainer_but_the_dummy_0_and_the_seen_slot", () => {
    expect(placeableTrainers(rows, "Pt").map((r) => r.index)).toEqual([4, 356, 928, 930]);
    // HeartGold's seen slot is 740: Platinum's 929 is an ordinary ID there.
    expect(placeableTrainers(rows, "Hgss").map((r) => r.index)).toEqual([4, 356, 928, 929, 930]);
  });

  it("names_trainers_by_class_and_name", () => {
    expect(trainerLabel(rows, 356)).toBe("Teenager Sebastian");
    expect(trainerLabel(rows, 777)).toBe("Trainer 0777");
    expect(trainerLabel(null, 356)).toBe("Trainer 0356");
    expect(scriptTrainerLabel(rows, 3355)).toBe("Teenager Sebastian");
    expect(scriptTrainerLabel(rows, 5003)).toBe("Teenager Michael (Partner)");
    expect(scriptTrainerLabel(rows, 4)).toBeNull();
  });

  it("names_the_undo_step_of_placing_by_the_trainers_own_name", () => {
    expect(placedLabel(rows, 928)).toBe("Trainer Jens platziert");
    expect(placedLabel(rows, 929)).toBe("Trainer 0929 platziert");
    expect(placedLabel(null, 930)).toBe("Trainer 0930 platziert");
  });
});
