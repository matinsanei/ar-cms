import { describe, it, expect, beforeAll } from 'vitest';

const BACKEND_URL = 'http://localhost:8088';

describe('Deep Authentication, Security & RBAC Suite', () => {
  let superAdminToken = '';
  let superAdminUser: any = null;
  let testRoleId = 0;
  let testUserId = 0;
  let testApiTokenId = 0;

  beforeAll(async () => {
    // Authenticate with Super Admin
    const loginRes = await fetch(`${BACKEND_URL}/admin/login`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ email: 'matin@pixelstack.io', password: 'Password123!' }),
    });
    expect(loginRes.status).toBe(200);
    const body = await loginRes.json();
    superAdminToken = body.data.token;
    superAdminUser = body.data.user;
  });

  describe('1. Authentication Validation & Error Handling', () => {
    it('rejects login with incorrect password', async () => {
      const res = await fetch(`${BACKEND_URL}/admin/login`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ email: 'matin@pixelstack.io', password: 'WrongPassword999!' }),
      });
      expect(res.status).toBe(400);
      const data = await res.json();
      expect(data.error.name).toBe('ValidationError');
      expect(data.error.message).toContain('Invalid email or password');
    });

    it('rejects login with unregistered email', async () => {
      const res = await fetch(`${BACKEND_URL}/admin/login`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ email: 'unknown_ghost@pixelstack.io', password: 'Password123!' }),
      });
      expect(res.status).toBe(400);
      const data = await res.json();
      expect(data.error.name).toBe('ValidationError');
    });

    it('rejects login with empty email or password', async () => {
      const res = await fetch(`${BACKEND_URL}/admin/login`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ email: '', password: '' }),
      });
      expect(res.status).toBe(400);
      const data = await res.json();
      expect(data.error.message).toContain('Email and password are required');
    });

    it('authenticates case-insensitively for email', async () => {
      const res = await fetch(`${BACKEND_URL}/admin/login`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ email: 'MATIN@PIXELSTACK.IO', password: 'Password123!' }),
      });
      expect(res.status).toBe(200);
      const data = await res.json();
      expect(data.data.token).toBeDefined();
      expect(data.data.user.email).toBe('matin@pixelstack.io');
    });

    it('sets HTTP-only jwtToken cookie on successful login', async () => {
      const res = await fetch(`${BACKEND_URL}/admin/login`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ email: 'matin@pixelstack.io', password: 'Password123!' }),
      });
      expect(res.status).toBe(200);
      const cookie = res.headers.get('set-cookie');
      expect(cookie).toBeDefined();
      expect(cookie).toContain('jwtToken=');
      expect(cookie).toContain('Path=/');
    });
  });

  describe('2. Admin User Profile & Session Management', () => {
    it('GET /admin/users/me returns currently logged-in user with sanitized fields', async () => {
      const res = await fetch(`${BACKEND_URL}/admin/users/me`, {
        headers: { Authorization: `Bearer ${superAdminToken}` },
      });
      expect(res.status).toBe(200);
      const data = await res.json();
      expect(data.data.email).toBe('matin@pixelstack.io');
      expect(data.data.password_hash).toBeUndefined(); // Security: never leak password hash
      expect(data.data.password).toBeUndefined();
    });

    it('PUT /admin/users/me updates current user profile', async () => {
      const res = await fetch(`${BACKEND_URL}/admin/users/me`, {
        method: 'PUT',
        headers: {
          'Content-Type': 'application/json',
          Authorization: `Bearer ${superAdminToken}`,
        },
        body: JSON.stringify({
          firstname: 'Matin Updated',
          lastname: 'Lead Architect',
        }),
      });
      expect(res.status).toBe(200);
      const data = await res.json();
      expect(data.data.firstname).toBe('Matin Updated');
      expect(data.data.lastname).toBe('Lead Architect');
    });

    it('POST /admin/renew-token generates fresh JWT token', async () => {
      const res = await fetch(`${BACKEND_URL}/admin/renew-token`, {
        method: 'POST',
        headers: { Authorization: `Bearer ${superAdminToken}` },
      });
      expect(res.status).toBe(200);
      const data = await res.json();
      expect(data.data.token).toBeDefined();
      expect(typeof data.data.token).toBe('string');
    });

    it('GET /admin/users/me/sessions returns active user sessions list', async () => {
      const res = await fetch(`${BACKEND_URL}/admin/users/me/sessions`, {
        headers: { Authorization: `Bearer ${superAdminToken}` },
      });
      expect(res.status).toBe(200);
      const data = await res.json();
      expect(Array.isArray(data.data)).toBe(true);
    });

    it('DELETE /admin/users/me/sessions revokes other active sessions', async () => {
      const res = await fetch(`${BACKEND_URL}/admin/users/me/sessions`, {
        method: 'DELETE',
        headers: { Authorization: `Bearer ${superAdminToken}` },
      });
      expect(res.status).toBe(200);
    });
  });

  describe('3. Admin RBAC Roles Management', () => {
    it('GET /admin/roles lists built-in super-admin and custom roles', async () => {
      const res = await fetch(`${BACKEND_URL}/admin/roles`, {
        headers: { Authorization: `Bearer ${superAdminToken}` },
      });
      expect(res.status).toBe(200);
      const data = await res.json();
      expect(Array.isArray(data.data)).toBe(true);
      const superAdmin = data.data.find((r: any) => r.code === 'strapi-super-admin');
      expect(superAdmin).toBeDefined();
    });

    it('POST /admin/roles creates custom role with description', async () => {
      const res = await fetch(`${BACKEND_URL}/admin/roles`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          Authorization: `Bearer ${superAdminToken}`,
        },
        body: JSON.stringify({
          name: 'Content Reviewer',
          code: `role-reviewer-${Date.now()}`,
          description: 'Responsible for reviewing content before publishing',
        }),
      });
      expect([200, 201]).toContain(res.status);
      const data = await res.json();
      expect(data.data.name).toBe('Content Reviewer');
      testRoleId = data.data.id;
    });

    it('GET /admin/roles/:id retrieves role details', async () => {
      const res = await fetch(`${BACKEND_URL}/admin/roles/${testRoleId}`, {
        headers: { Authorization: `Bearer ${superAdminToken}` },
      });
      expect(res.status).toBe(200);
      const data = await res.json();
      expect(data.data.id).toBe(testRoleId);
      expect(data.data.name).toBe('Content Reviewer');
    });

    it('PUT /admin/roles/:id updates role properties', async () => {
      const res = await fetch(`${BACKEND_URL}/admin/roles/${testRoleId}`, {
        method: 'PUT',
        headers: {
          'Content-Type': 'application/json',
          Authorization: `Bearer ${superAdminToken}`,
        },
        body: JSON.stringify({
          name: 'Senior Content Reviewer',
          description: 'Updated senior privileges',
        }),
      });
      expect(res.status).toBe(200);
      const data = await res.json();
      expect(data.data.name).toBe('Senior Content Reviewer');
    });

    it('GET and PUT /admin/roles/:id/permissions manages role permissions catalog', async () => {
      const getRes = await fetch(`${BACKEND_URL}/admin/roles/${testRoleId}/permissions`, {
        headers: { Authorization: `Bearer ${superAdminToken}` },
      });
      expect(getRes.status).toBe(200);

      const putRes = await fetch(`${BACKEND_URL}/admin/roles/${testRoleId}/permissions`, {
        method: 'PUT',
        headers: {
          'Content-Type': 'application/json',
          Authorization: `Bearer ${superAdminToken}`,
        },
        body: JSON.stringify({
          permissions: [
            { action: 'plugin::content-manager.explorer.read' },
            { action: 'plugin::content-manager.explorer.create' },
          ],
        }),
      });
      expect(putRes.status).toBe(200);
    });

    it('DELETE /admin/roles/:id removes custom role', async () => {
      const res = await fetch(`${BACKEND_URL}/admin/roles/${testRoleId}`, {
        method: 'DELETE',
        headers: { Authorization: `Bearer ${superAdminToken}` },
      });
      expect(res.status).toBe(200);
    });
  });

  describe('4. Admin Users Management & Batch Deletion', () => {
    const newUserEmail = `testuser_${Date.now()}@pixelstack.io`;

    it('POST /admin/users creates a secondary admin user', async () => {
      const res = await fetch(`${BACKEND_URL}/admin/users`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          Authorization: `Bearer ${superAdminToken}`,
        },
        body: JSON.stringify({
          firstname: 'Sara',
          lastname: 'Designer',
          email: newUserEmail,
          password: 'Password123!',
          roles: [1],
        }),
      });
      expect([200, 201]).toContain(res.status);
      const data = await res.json();
      expect(data.data.email).toBe(newUserEmail);
      testUserId = data.data.id;
    });

    it('GET /admin/users lists all registered admin users with pagination', async () => {
      const res = await fetch(`${BACKEND_URL}/admin/users?pageSize=10&page=1`, {
        headers: { Authorization: `Bearer ${superAdminToken}` },
      });
      expect(res.status).toBe(200);
      const data = await res.json();
      expect(data.data.results).toBeDefined();
      expect(data.data.pagination.total).toBeGreaterThanOrEqual(2);
    });

    it('GET /admin/users/:id retrieves specific admin user', async () => {
      const res = await fetch(`${BACKEND_URL}/admin/users/${testUserId}`, {
        headers: { Authorization: `Bearer ${superAdminToken}` },
      });
      expect(res.status).toBe(200);
      const data = await res.json();
      expect(data.data.email).toBe(newUserEmail);
    });

    it('PUT /admin/users/:id updates admin user details', async () => {
      const res = await fetch(`${BACKEND_URL}/admin/users/${testUserId}`, {
        method: 'PUT',
        headers: {
          'Content-Type': 'application/json',
          Authorization: `Bearer ${superAdminToken}`,
        },
        body: JSON.stringify({
          firstname: 'Sara Updated',
          lastname: 'Head of Design',
        }),
      });
      expect(res.status).toBe(200);
      const data = await res.json();
      expect(data.data.firstname).toBe('Sara Updated');
    });

    it('POST /admin/users/batch-delete deletes multiple users at once', async () => {
      const res = await fetch(`${BACKEND_URL}/admin/users/batch-delete`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          Authorization: `Bearer ${superAdminToken}`,
        },
        body: JSON.stringify({
          ids: [testUserId],
        }),
      });
      expect(res.status).toBe(200);
      const data = await res.json();
      expect(data.data.count).toBe(1);
    });
  });

  describe('5. API & Transfer Tokens Management', () => {
    it('POST /admin/api-tokens creates a read-only API token', async () => {
      const res = await fetch(`${BACKEND_URL}/admin/api-tokens`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          Authorization: `Bearer ${superAdminToken}`,
        },
        body: JSON.stringify({
          name: 'Storefront Readonly Token',
          description: 'Used by public web store',
          type: 'read-only',
          lifespan: null,
        }),
      });
      expect([200, 201]).toContain(res.status);
      const data = await res.json();
      expect(data.data.name).toBe('Storefront Readonly Token');
      expect(data.data.accessKey).toBeDefined();
      testApiTokenId = data.data.id;
    });

    it('POST /admin/api-tokens/:id/regenerate generates new access key', async () => {
      const res = await fetch(`${BACKEND_URL}/admin/api-tokens/${testApiTokenId}/regenerate`, {
        method: 'POST',
        headers: { Authorization: `Bearer ${superAdminToken}` },
      });
      expect([200, 201]).toContain(res.status);
      const data = await res.json();
      expect(data.data.accessKey).toBeDefined();
    });

    it('DELETE /admin/api-tokens/:id deletes API token', async () => {
      const res = await fetch(`${BACKEND_URL}/admin/api-tokens/${testApiTokenId}`, {
        method: 'DELETE',
        headers: { Authorization: `Bearer ${superAdminToken}` },
      });
      expect(res.status).toBe(200);
    });
  });

  describe('6. Audit Logging Verification', () => {
    it('GET /admin/audit-logs contains security log entries', async () => {
      const res = await fetch(`${BACKEND_URL}/admin/audit-logs`, {
        headers: { Authorization: `Bearer ${superAdminToken}` },
      });
      expect(res.status).toBe(200);
      const data = await res.json();
      expect(Array.isArray(data.data)).toBe(true);
      expect(data.data.length).toBeGreaterThan(0);
      const loginEvent = data.data.find((l: any) => l.action === 'admin.login' && l.userEmail === 'matin@pixelstack.io');
      expect(loginEvent).toBeDefined();
    });
  });
});
