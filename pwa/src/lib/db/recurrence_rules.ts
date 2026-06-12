import { getDatabase } from './index';
import type { RecurrenceRuleRecord } from './schema';

export async function createRecurrenceRule(rule: RecurrenceRuleRecord): Promise<void> {
  const db = await getDatabase();
  await db.put('recurrence_rules', rule);
}

export async function getRecurrenceRule(id: string): Promise<RecurrenceRuleRecord | undefined> {
  const db = await getDatabase();
  return db.get('recurrence_rules', id);
}

export async function getAllRecurrenceRules(): Promise<RecurrenceRuleRecord[]> {
  const db = await getDatabase();
  return db.getAll('recurrence_rules');
}
