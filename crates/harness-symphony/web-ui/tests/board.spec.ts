import { expect, test } from "@playwright/test";

test("board renders task columns and detail controls", async ({ page }) => {
  await page.goto("/");

  await expect(page.getByRole("heading", { name: "Symphony Controller" })).toBeVisible();
  await expect(page.getByRole("heading", { name: "Ready", exact: true })).toBeVisible();
  await expect(page.getByRole("heading", { name: "Blocked", exact: true })).toBeVisible();
  await expect(page.getByRole("heading", { name: "In Progress", exact: true })).toBeVisible();
  await expect(page.getByRole("heading", { name: "Review", exact: true })).toBeVisible();
  await expect(page.getByRole("heading", { name: "Needs Attention", exact: true })).toBeVisible();
  await expect(page.getByRole("heading", { name: "Done", exact: true })).toBeVisible();

  await page.getByPlaceholder("Find task").fill("US-052");
  await expect(page.getByRole("button", { name: /US-052/ })).toBeVisible();
  await page.getByRole("button", { name: /US-052/ }).click();

  const detail = page.getByRole("complementary");
  await expect(
    detail.getByRole("heading", { name: "Sync Approval And Done Transition" })
  ).toBeVisible();
  await expect(page.getByText("Blocked by")).toBeVisible();
  await expect(page.getByText("Unblocks")).toBeVisible();
  await expect(detail.getByText("Hierarchy")).toBeVisible();
  await expect(detail.getByRole("button", { name: "Start" })).toBeVisible();
});
