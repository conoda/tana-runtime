/**
 * Team Account Service
 *
 * CRUD operations for teams and team membership
 */

import { eq, and } from 'drizzle-orm'
import { db, teams, teamMembers } from '../db'

export interface CreateTeamInput {
  name: string
  slug: string
  description?: string
  ownerId: string // User ID of team creator
}

export interface UpdateTeamInput {
  name?: string
  description?: string
  avatarData?: string
  landingPageId?: string
}

export interface AddMemberInput {
  teamId: string
  userId: string
  role?: 'owner' | 'admin' | 'member'
}

/**
 * Create a new team
 */
export async function createTeam(input: CreateTeamInput) {
  // Validate slug format (@acme)
  if (!input.slug.startsWith('@')) {
    throw new Error('Team slug must start with @')
  }

  // Create team
  const [team] = await db
    .insert(teams)
    .values({
      name: input.name,
      slug: input.slug,
      description: input.description,
    })
    .returning()

  // Add creator as owner
  await db.insert(teamMembers).values({
    teamId: team.id,
    userId: input.ownerId,
    role: 'owner',
  })

  return team
}

/**
 * Get team by ID
 */
export async function getTeamById(id: string) {
  const [team] = await db.select().from(teams).where(eq(teams.id, id)).limit(1)
  return team || null
}

/**
 * Get team by slug
 */
export async function getTeamBySlug(slug: string) {
  const [team] = await db.select().from(teams).where(eq(teams.slug, slug)).limit(1)
  return team || null
}

/**
 * Update team
 */
export async function updateTeam(id: string, input: UpdateTeamInput) {
  const [updated] = await db
    .update(teams)
    .set({
      ...input,
      updatedAt: new Date(),
    })
    .where(eq(teams.id, id))
    .returning()

  return updated || null
}

/**
 * Delete team
 */
export async function deleteTeam(id: string) {
  const [deleted] = await db.delete(teams).where(eq(teams.id, id)).returning()
  return deleted || null
}

/**
 * List all teams (paginated)
 */
export async function listTeams(limit = 50, offset = 0) {
  return await db.select().from(teams).limit(limit).offset(offset)
}

// ============================================================================
// TEAM MEMBERSHIP
// ============================================================================

/**
 * Add member to team
 */
export async function addTeamMember(input: AddMemberInput) {
  const [member] = await db
    .insert(teamMembers)
    .values({
      teamId: input.teamId,
      userId: input.userId,
      role: input.role || 'member',
    })
    .returning()

  return member
}

/**
 * Remove member from team
 */
export async function removeTeamMember(teamId: string, userId: string) {
  const [removed] = await db
    .delete(teamMembers)
    .where(and(eq(teamMembers.teamId, teamId), eq(teamMembers.userId, userId)))
    .returning()

  return removed || null
}

/**
 * Update member role
 */
export async function updateMemberRole(teamId: string, userId: string, role: 'owner' | 'admin' | 'member') {
  const [updated] = await db
    .update(teamMembers)
    .set({ role })
    .where(and(eq(teamMembers.teamId, teamId), eq(teamMembers.userId, userId)))
    .returning()

  return updated || null
}

/**
 * Get team members
 */
export async function getTeamMembers(teamId: string) {
  return await db.select().from(teamMembers).where(eq(teamMembers.teamId, teamId))
}

/**
 * Get user's teams
 */
export async function getUserTeams(userId: string) {
  return await db.select().from(teamMembers).where(eq(teamMembers.userId, userId))
}

/**
 * Check if user is team member
 */
export async function isTeamMember(teamId: string, userId: string): Promise<boolean> {
  const [member] = await db
    .select()
    .from(teamMembers)
    .where(and(eq(teamMembers.teamId, teamId), eq(teamMembers.userId, userId)))
    .limit(1)

  return !!member
}

/**
 * Check if user has role in team
 */
export async function hasTeamRole(
  teamId: string,
  userId: string,
  requiredRole: 'owner' | 'admin' | 'member'
): Promise<boolean> {
  const [member] = await db
    .select()
    .from(teamMembers)
    .where(and(eq(teamMembers.teamId, teamId), eq(teamMembers.userId, userId)))
    .limit(1)

  if (!member) return false

  // Role hierarchy: owner > admin > member
  const roleHierarchy: Record<string, number> = {
    owner: 3,
    admin: 2,
    member: 1,
  }

  return roleHierarchy[member.role] >= roleHierarchy[requiredRole]
}
